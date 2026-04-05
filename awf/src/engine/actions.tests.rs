//! Tests for [`super::next_action`] and [`super::can_advance`].
//!
//! All tests use the canonical "feature" workflow fixture with three stages:
//! `design` → `implement` → `review`.

use std::collections::HashMap;
use std::collections::HashSet;

use chrono::DateTime;
use chrono::Utc;

use super::AdvanceBlocker;
use super::NextAction;
use super::can_advance;
use super::next_action;
use crate::model::flow::FlowState;
use crate::model::flow::StepLogEntry;
use crate::model::flow::StepStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::Step;
use crate::model::workflow::StepKind;
use crate::model::workflow::Workflow;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Builds the canonical "feature" workflow used across all tests.
///
/// Stages:
/// - `design`: agent step `write-design`; produces `design.md`
/// - `implement`: agent step `write-code`, command steps `format` and `lint`
/// - `review` (repeatable): command step `pre-review-test`, agent step
///   `do-review`; produces `changelog-entry.md`
fn feature_workflow() -> Workflow {
    Workflow {
        name: "feature".to_owned(),
        version: 1,
        description: Some(
            "Ship a new feature from design through merge.".to_owned(),
        ),
        commands: {
            let mut m = HashMap::new();
            m.insert("format".to_owned(), "cargo fmt --check".to_owned());
            m.insert(
                "lint".to_owned(),
                "cargo clippy -- -D warnings".to_owned(),
            );
            m.insert("test".to_owned(), "cargo test --workspace".to_owned());
            m
        },
        stages: vec![
            Stage {
                name: "design".to_owned(),
                description: Some("Produce a design doc.".to_owned()),
                repeatable: false,
                produces: vec!["design.md".to_owned()],
                requires: vec![],
                steps: vec![Step {
                    name: "write-design".to_owned(),
                    kind: StepKind::Agent {
                        prompt: Some("design.md".to_owned()),
                        instructions: None,
                    },
                }],
            },
            Stage {
                name: "implement".to_owned(),
                description: Some("Write the code.".to_owned()),
                repeatable: false,
                produces: vec![],
                requires: vec!["design.md".to_owned()],
                steps: vec![
                    Step {
                        name: "write-code".to_owned(),
                        kind: StepKind::Agent {
                            prompt: Some("implement.md".to_owned()),
                            instructions: None,
                        },
                    },
                    Step {
                        name: "format".to_owned(),
                        kind: StepKind::Command {
                            command: Some("format".to_owned()),
                            run: None,
                        },
                    },
                    Step {
                        name: "lint".to_owned(),
                        kind: StepKind::Command {
                            command: Some("lint".to_owned()),
                            run: None,
                        },
                    },
                ],
            },
            Stage {
                name: "review".to_owned(),
                description: None,
                repeatable: true,
                produces: vec!["changelog-entry.md".to_owned()],
                requires: vec![],
                steps: vec![
                    Step {
                        name: "pre-review-test".to_owned(),
                        kind: StepKind::Command {
                            command: Some("test".to_owned()),
                            run: None,
                        },
                    },
                    Step {
                        name: "do-review".to_owned(),
                        kind: StepKind::Agent {
                            prompt: Some("review.md".to_owned()),
                            instructions: None,
                        },
                    },
                ],
            },
        ],
        default_flow_dir: None,
    }
}

/// Builds a minimal [`FlowState`] for the "feature" workflow at the given
/// stage with the provided step log entries.
fn flow_at(stage: &str, step_log: Vec<StepLogEntry>) -> FlowState {
    FlowState {
        id: "add-oauth".to_owned(),
        workflow: "feature".to_owned(),
        workflow_version: 1,
        current_stage: stage.to_owned(),
        stage_entered_at: ts("2026-04-04T10:00:00Z"),
        created_at: ts("2026-04-04T10:00:00Z"),
        updated_at: ts("2026-04-04T10:00:00Z"),
        archived: false,
        stage_history: vec![],
        step_log,
    }
}

/// Builds a passing [`StepLogEntry`] for a command step.
fn pass_cmd(stage: &str, step: &str) -> StepLogEntry {
    StepLogEntry {
        step: step.to_owned(),
        stage: stage.to_owned(),
        step_type: "command".to_owned(),
        status: StepStatus::Pass,
        exit_code: Some(0),
        started_at: Some(ts("2026-04-04T14:00:00Z")),
        duration_ms: Some(500),
        completed_at: None,
    }
}

/// Builds a failing [`StepLogEntry`] for a command step.
fn fail_cmd(stage: &str, step: &str) -> StepLogEntry {
    StepLogEntry {
        step: step.to_owned(),
        stage: stage.to_owned(),
        step_type: "command".to_owned(),
        status: StepStatus::Fail,
        exit_code: Some(1),
        started_at: Some(ts("2026-04-04T14:00:00Z")),
        duration_ms: Some(800),
        completed_at: None,
    }
}

/// Builds a passing [`StepLogEntry`] for an agent step.
fn pass_agent(stage: &str, step: &str) -> StepLogEntry {
    StepLogEntry {
        step: step.to_owned(),
        stage: stage.to_owned(),
        step_type: "agent".to_owned(),
        status: StepStatus::Pass,
        exit_code: None,
        started_at: None,
        duration_ms: None,
        completed_at: Some(ts("2026-04-04T13:00:00Z")),
    }
}

/// Parse an RFC 3339 timestamp into [`DateTime<Utc>`], panicking on failure.
fn ts(s: &str) -> DateTime<Utc> { s.parse().expect("timestamp should parse") }

// ---------------------------------------------------------------------------
// next_action — NextAction::RunStep (failing command step, priority 1)
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_run_step_for_failing_command_step() {
    let workflow = feature_workflow();

    // Given: the implement stage, where `format` failed and `lint` has not
    // run yet
    let flow = flow_at("implement", vec![
        pass_agent("implement", "write-code"),
        fail_cmd("implement", "format"),
    ]);

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &HashSet::new());

    // Then: the failing command step is returned (priority 1 beats priority 2)
    let format_step = &workflow.stages[1].steps[1]; // "format"
    assert_eq!(action, NextAction::RunStep(format_step));
}

// ---------------------------------------------------------------------------
// next_action — NextAction::RunStep (pending command step, priority 2)
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_run_step_for_pending_command_step() {
    let workflow = feature_workflow();

    // Given: the implement stage, where the agent step passed but neither
    // command step has run yet
    let flow =
        flow_at("implement", vec![pass_agent("implement", "write-code")]);

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &HashSet::new());

    // Then: the first incomplete command step is returned
    let format_step = &workflow.stages[1].steps[1]; // "format"
    assert_eq!(action, NextAction::RunStep(format_step));
}

// ---------------------------------------------------------------------------
// next_action — NextAction::CompleteAgentStep
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_complete_agent_step_for_pending_agent_step() {
    let workflow = feature_workflow();

    // Given: the implement stage with no steps completed
    let flow = flow_at("implement", vec![]);

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &HashSet::new());

    // Then: the first incomplete agent step is returned
    let write_code_step = &workflow.stages[1].steps[0]; // "write-code"
    assert_eq!(action, NextAction::CompleteAgentStep(write_code_step));
}

// ---------------------------------------------------------------------------
// next_action — NextAction::ProduceArtifact
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_produce_artifact_when_artifact_missing() {
    let workflow = feature_workflow();

    // Given: the design stage with the agent step completed, but the
    // `design.md` artifact not yet registered
    let flow = flow_at("design", vec![pass_agent("design", "write-design")]);
    let artifacts = HashSet::new(); // design.md absent

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &artifacts);

    // Then: we are told to produce the missing artifact
    assert_eq!(action, NextAction::ProduceArtifact("design.md".to_owned()));
}

// ---------------------------------------------------------------------------
// next_action — NextAction::CanAdvance
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_can_advance_when_stage_is_complete_and_not_last() {
    let workflow = feature_workflow();

    // Given: the design stage is fully done (step passed, artifact present),
    // and this is not the last stage
    let flow = flow_at("design", vec![pass_agent("design", "write-design")]);
    let mut artifacts = HashSet::new();
    artifacts.insert("design.md".to_owned());

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &artifacts);

    // Then: we are told the stage is ready to advance
    assert_eq!(action, NextAction::CanAdvance);
}

// ---------------------------------------------------------------------------
// next_action — NextAction::Done
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_done_when_last_stage_is_complete() {
    let workflow = feature_workflow();

    // Given: the review stage (last stage) is fully done — both steps passed
    // and the `changelog-entry.md` artifact is present
    let flow = flow_at("review", vec![
        pass_cmd("review", "pre-review-test"),
        pass_agent("review", "do-review"),
    ]);
    let mut artifacts = HashSet::new();
    artifacts.insert("changelog-entry.md".to_owned());

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &artifacts);

    // Then: the workflow is done — nothing more to do
    assert_eq!(action, NextAction::Done);
}

// ---------------------------------------------------------------------------
// can_advance — StepFailed blocker
// ---------------------------------------------------------------------------

#[test]
fn can_advance_returns_step_failed_when_last_entry_for_step_is_fail() {
    let workflow = feature_workflow();

    // Given: the implement stage where `format` passed but `lint` failed
    let flow = flow_at("implement", vec![
        pass_agent("implement", "write-code"),
        pass_cmd("implement", "format"),
        fail_cmd("implement", "lint"),
    ]);

    // When: we check whether the stage can advance
    let result = can_advance(&workflow, &flow, &HashSet::new());

    // Then: blocked because `lint` has failed
    assert_eq!(result, Err(AdvanceBlocker::StepFailed("lint".to_owned())));
}

// ---------------------------------------------------------------------------
// can_advance — StepIncomplete blocker
// ---------------------------------------------------------------------------

#[test]
fn can_advance_returns_step_incomplete_when_step_has_no_pass_entry() {
    let workflow = feature_workflow();

    // Given: the implement stage where `write-code` passed but `format` has
    // never been run
    let flow =
        flow_at("implement", vec![pass_agent("implement", "write-code")]);

    // When: we check whether the stage can advance
    let result = can_advance(&workflow, &flow, &HashSet::new());

    // Then: blocked because `format` has no pass entry
    assert_eq!(
        result,
        Err(AdvanceBlocker::StepIncomplete("format".to_owned()))
    );
}

// ---------------------------------------------------------------------------
// can_advance — MissingArtifact blocker
// ---------------------------------------------------------------------------

#[test]
fn can_advance_returns_missing_artifact_when_produces_artifact_is_absent() {
    let workflow = feature_workflow();

    // Given: the design stage where the agent step passed, but the produced
    // artifact has not been registered
    let flow = flow_at("design", vec![pass_agent("design", "write-design")]);
    let artifacts = HashSet::new(); // design.md not present

    // When: we check whether the stage can advance
    let result = can_advance(&workflow, &flow, &artifacts);

    // Then: blocked because `design.md` is missing
    assert_eq!(
        result,
        Err(AdvanceBlocker::MissingArtifact("design.md".to_owned()))
    );
}

// ---------------------------------------------------------------------------
// can_advance — success
// ---------------------------------------------------------------------------

#[test]
fn can_advance_returns_ok_when_all_steps_pass_and_artifacts_present() {
    let workflow = feature_workflow();

    // Given: the design stage where the agent step passed and the artifact is
    // registered
    let flow = flow_at("design", vec![pass_agent("design", "write-design")]);
    let mut artifacts = HashSet::new();
    artifacts.insert("design.md".to_owned());

    // When: we check whether the stage can advance
    let result = can_advance(&workflow, &flow, &artifacts);

    // Then: no blockers — the stage may advance
    assert_eq!(result, Ok(()));
}

// ---------------------------------------------------------------------------
// can_advance — step passes after earlier failure (re-run scenario)
// ---------------------------------------------------------------------------

#[test]
fn can_advance_passes_when_step_reruns_and_last_entry_is_pass() {
    let workflow = feature_workflow();

    // Given: the implement stage where `format` initially failed, was re-run,
    // and then passed; all other steps also passed
    let flow = flow_at("implement", vec![
        pass_agent("implement", "write-code"),
        fail_cmd("implement", "format"), // first attempt: fail
        pass_cmd("implement", "format"), // second attempt: pass
        pass_cmd("implement", "lint"),
    ]);

    // When: we check whether the stage can advance (implement has no produces)
    let result = can_advance(&workflow, &flow, &HashSet::new());

    // Then: the stage can advance — the last entry for `format` is a pass
    assert_eq!(result, Ok(()));
}
