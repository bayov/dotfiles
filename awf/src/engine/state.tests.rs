//! Tests for [`super::can_enter_stage`], [`super::advance_stage`],
//! [`super::record_step_pass`], and [`super::record_step_fail`].
//!
//! All tests use the canonical "feature" workflow fixture with three stages:
//! `design` → `implement` → `review`.

use std::collections::HashMap;
use std::collections::HashSet;

use chrono::DateTime;
use chrono::Utc;

use super::EnterResult;
use super::advance_stage;
use super::can_enter_stage;
use super::record_step_fail;
use super::record_step_pass;
use crate::model::flow::FlowState;
use crate::model::flow::StageHistoryEntry;
use crate::model::flow::StepStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::Step;
use crate::model::workflow::StepKind;
use crate::model::workflow::Workflow;
use crate::result::Error;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Builds the canonical "feature" workflow used across all tests.
///
/// Stages:
/// - `design` (requires nothing, produces `design.md`): agent step
///   `write-design`
/// - `implement` (requires `design.md`, no produces): agent step `write-code`,
///   command steps `format` and `lint`
/// - `review` (repeatable, produces `changelog-entry.md`): command step
///   `pre-review-test`, agent step `do-review`
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
    }
}

/// Builds a minimal [`FlowState`] for the "feature" workflow at the given
/// stage with no step log and no stage history.
fn flow_at(stage: &str) -> FlowState {
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
        step_log: vec![],
    }
}

/// Builds a minimal [`Step`] with a [`StepKind::Agent`] kind.
fn agent_step(name: &str) -> Step {
    Step {
        name: name.to_owned(),
        kind: StepKind::Agent {
            prompt: Some(format!("{name}.md")),
            instructions: None,
        },
    }
}

/// Builds a minimal [`Step`] with a [`StepKind::Command`] kind.
fn command_step(name: &str) -> Step {
    Step {
        name: name.to_owned(),
        kind: StepKind::Command {
            run: Some(format!("run-{name}")),
            command: None,
        },
    }
}

/// Parse an RFC 3339 timestamp into [`DateTime<Utc>`], panicking on failure.
fn ts(s: &str) -> DateTime<Utc> { s.parse().expect("timestamp should parse") }

// ---------------------------------------------------------------------------
// can_enter_stage — Ok
// ---------------------------------------------------------------------------

#[test]
fn can_enter_stage_returns_ok_on_first_visit_with_no_requirements() {
    let workflow = feature_workflow();
    let flow = flow_at("design");
    let artifacts = HashSet::new();

    // Given: the design stage has no `requires`, and has never been visited
    // When: we check if we can enter the design stage
    let result = can_enter_stage(&workflow, &flow, "design", &artifacts);

    // Then: the stage may be entered
    assert_eq!(result, EnterResult::Ok);
}

#[test]
fn can_enter_stage_returns_ok_when_required_artifacts_are_present() {
    let workflow = feature_workflow();
    let flow = flow_at("implement");
    let mut artifacts = HashSet::new();
    artifacts.insert("design.md".to_owned());

    // Given: the implement stage requires `design.md`, which is present
    // When: we check if we can enter the implement stage
    let result = can_enter_stage(&workflow, &flow, "implement", &artifacts);

    // Then: the stage may be entered
    assert_eq!(result, EnterResult::Ok);
}

// ---------------------------------------------------------------------------
// can_enter_stage — MissingRequires
// ---------------------------------------------------------------------------

#[test]
fn can_enter_stage_returns_missing_requires_when_artifact_absent() {
    let workflow = feature_workflow();
    let flow = flow_at("design");
    let artifacts = HashSet::new(); // design.md not present

    // Given: the implement stage requires `design.md`, which is absent
    // When: we check if we can enter the implement stage
    let result = can_enter_stage(&workflow, &flow, "implement", &artifacts);

    // Then: MissingRequires lists the missing artifact
    assert_eq!(
        result,
        EnterResult::MissingRequires(vec!["design.md".to_owned()])
    );
}

// ---------------------------------------------------------------------------
// can_enter_stage — AlreadyCompleted (non-repeatable)
// ---------------------------------------------------------------------------

#[test]
fn can_enter_stage_returns_already_completed_for_non_repeatable_visited_stage()
{
    let workflow = feature_workflow();

    // Given: design has been completed and appears in stage_history
    let mut flow = flow_at("implement");
    flow.stage_history.push(StageHistoryEntry {
        stage: "design".to_owned(),
        entered_at: ts("2026-04-04T10:00:00Z"),
        exited_at: Some(ts("2026-04-04T12:00:00Z")),
    });

    let mut artifacts = HashSet::new();
    artifacts.insert("design.md".to_owned()); // requirement satisfied

    // When: we check if we can re-enter the design stage (repeatable = false)
    let result = can_enter_stage(&workflow, &flow, "design", &artifacts);

    // Then: AlreadyCompleted is returned with design's produces list
    assert_eq!(result, EnterResult::AlreadyCompleted {
        produces: vec!["design.md".to_owned()],
    });
}

// ---------------------------------------------------------------------------
// can_enter_stage — repeatable stage is always Ok if requirements met
// ---------------------------------------------------------------------------

#[test]
fn can_enter_stage_returns_ok_for_repeatable_stage_even_if_in_history() {
    let workflow = feature_workflow();

    // Given: review has already been visited and appears in stage_history
    let mut flow = flow_at("review");
    flow.stage_history.push(StageHistoryEntry {
        stage: "review".to_owned(),
        entered_at: ts("2026-04-04T10:00:00Z"),
        exited_at: Some(ts("2026-04-04T11:00:00Z")),
    });

    let artifacts = HashSet::new(); // review has no requires

    // When: we check if we can enter the review stage again (repeatable = true)
    let result = can_enter_stage(&workflow, &flow, "review", &artifacts);

    // Then: the stage may be entered again
    assert_eq!(result, EnterResult::Ok);
}

// ---------------------------------------------------------------------------
// advance_stage — normal advance
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_moves_flow_from_design_to_implement() {
    let workflow = feature_workflow();

    // Given: a flow at the design stage, entered at a known time
    let mut flow = flow_at("design");
    flow.stage_entered_at = ts("2026-04-04T10:00:00Z");

    // When: we advance the stage
    let updated =
        advance_stage(&workflow, flow).expect("advance should succeed");

    // Then: the current stage is now implement
    assert_eq!(updated.current_stage, "implement");

    // Then: the stage history records the completed design stage
    assert_eq!(updated.stage_history.len(), 1);
    let history_entry = &updated.stage_history[0];
    assert_eq!(history_entry.stage, "design");
    assert_eq!(history_entry.entered_at, ts("2026-04-04T10:00:00Z"));
    assert!(
        history_entry.exited_at.is_some(),
        "exited_at should be set after advancing"
    );
}

#[test]
fn advance_stage_sets_stage_entered_at_for_new_stage() {
    let workflow = feature_workflow();
    let flow = flow_at("design");

    // When: we advance the stage
    let before = Utc::now();
    let updated =
        advance_stage(&workflow, flow).expect("advance should succeed");
    let after = Utc::now();

    // Then: stage_entered_at is set to a time within the current call window
    assert!(
        updated.stage_entered_at >= before && updated.stage_entered_at <= after,
        "stage_entered_at should be within the advance_stage call window"
    );
}

#[test]
fn advance_stage_updates_updated_at() {
    let workflow = feature_workflow();
    let flow = flow_at("design");
    let original_updated_at = flow.updated_at;

    // When: we advance the stage
    let updated =
        advance_stage(&workflow, flow).expect("advance should succeed");

    // Then: updated_at is refreshed (same instant as stage_entered_at)
    assert_eq!(updated.updated_at, updated.stage_entered_at);
    assert!(
        updated.updated_at >= original_updated_at,
        "updated_at should not move backwards"
    );
}

// ---------------------------------------------------------------------------
// advance_stage — last stage error
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_returns_already_on_last_stage_when_on_last_stage() {
    let workflow = feature_workflow();

    // Given: a flow at the review stage (last stage)
    let flow = flow_at("review");

    // When: we attempt to advance further
    let result = advance_stage(&workflow, flow);

    // Then: AlreadyOnLastStage error is returned
    assert!(
        matches!(result, Err(Error::AlreadyOnLastStage)),
        "expected Err(AlreadyOnLastStage), got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// record_step_pass — command step
// ---------------------------------------------------------------------------

#[test]
fn record_step_pass_appends_passing_command_entry_to_step_log() {
    let flow = flow_at("implement");
    let step = command_step("format");

    // Given: the implement stage flow and a command step named "format"
    // When: we record a passing run (exit code 0, 450 ms)
    let updated = record_step_pass(
        flow,
        &step,
        "implement",
        Some(0),
        Some(ts("2026-04-04T14:00:00Z")),
        Some(450),
    );

    // Then: one entry is in the step log
    assert_eq!(updated.step_log.len(), 1);
    let entry = &updated.step_log[0];

    assert_eq!(entry.step, "format");
    assert_eq!(entry.stage, "implement");
    assert_eq!(entry.step_type, "command");
    assert_eq!(entry.status, StepStatus::Pass);
    assert_eq!(entry.exit_code, Some(0));
    assert_eq!(entry.started_at, Some(ts("2026-04-04T14:00:00Z")));
    assert_eq!(entry.duration_ms, Some(450));
    assert!(entry.completed_at.is_none());
}

// ---------------------------------------------------------------------------
// record_step_fail — command step
// ---------------------------------------------------------------------------

#[test]
fn record_step_fail_appends_failing_command_entry_to_step_log() {
    let flow = flow_at("implement");
    let step = command_step("lint");

    // Given: the implement stage flow and a command step named "lint"
    // When: we record a failing run (exit code 1, 800 ms)
    let updated = record_step_fail(
        flow,
        &step,
        "implement",
        Some(1),
        Some(ts("2026-04-04T14:05:00Z")),
        Some(800),
    );

    // Then: one failing entry is in the step log
    assert_eq!(updated.step_log.len(), 1);
    let entry = &updated.step_log[0];

    assert_eq!(entry.step, "lint");
    assert_eq!(entry.stage, "implement");
    assert_eq!(entry.step_type, "command");
    assert_eq!(entry.status, StepStatus::Fail);
    assert_eq!(entry.exit_code, Some(1));
    assert_eq!(entry.started_at, Some(ts("2026-04-04T14:05:00Z")));
    assert_eq!(entry.duration_ms, Some(800));
    assert!(entry.completed_at.is_none());
}

// ---------------------------------------------------------------------------
// record_step_pass — agent step
// ---------------------------------------------------------------------------

#[test]
fn record_step_pass_appends_passing_agent_entry_with_completed_at() {
    let flow = flow_at("design");
    let step = agent_step("write-design");

    // Given: the design stage flow and an agent step named "write-design"
    // When: we record a passing agent completion (command fields are None)
    let before = Utc::now();
    let updated = record_step_pass(flow, &step, "design", None, None, None);
    let after = Utc::now();

    // Then: one passing agent entry is in the step log
    assert_eq!(updated.step_log.len(), 1);
    let entry = &updated.step_log[0];

    assert_eq!(entry.step, "write-design");
    assert_eq!(entry.stage, "design");
    assert_eq!(entry.step_type, "agent");
    assert_eq!(entry.status, StepStatus::Pass);
    assert!(entry.exit_code.is_none());
    assert!(entry.started_at.is_none());
    assert!(entry.duration_ms.is_none());

    // Then: completed_at is set to a time within the record_step_pass call
    let completed_at =
        entry.completed_at.expect("agent step should have completed_at set");
    assert!(
        completed_at >= before && completed_at <= after,
        "completed_at should be within the record_step_pass call window"
    );
}

// ---------------------------------------------------------------------------
// record_step_fail — agent step
// ---------------------------------------------------------------------------

#[test]
fn record_step_fail_appends_failing_agent_entry_with_completed_at() {
    let flow = flow_at("design");
    let step = agent_step("write-design");

    // Given: the design stage flow and an agent step named "write-design"
    // When: we record a failing agent completion
    let before = Utc::now();
    let updated = record_step_fail(flow, &step, "design", None, None, None);
    let after = Utc::now();

    // Then: one failing agent entry is in the step log
    assert_eq!(updated.step_log.len(), 1);
    let entry = &updated.step_log[0];

    assert_eq!(entry.step, "write-design");
    assert_eq!(entry.step_type, "agent");
    assert_eq!(entry.status, StepStatus::Fail);
    assert!(entry.exit_code.is_none());

    // Then: completed_at is set
    let completed_at =
        entry.completed_at.expect("agent step should have completed_at set");
    assert!(
        completed_at >= before && completed_at <= after,
        "completed_at should be within the record_step_fail call window"
    );
}

// ---------------------------------------------------------------------------
// multiple entries accumulate in step_log
// ---------------------------------------------------------------------------

#[test]
fn multiple_record_calls_accumulate_entries_in_step_log() {
    let flow = flow_at("implement");
    let write_code = agent_step("write-code");
    let format_step = command_step("format");
    let lint_step = command_step("lint");

    // Given: several steps being recorded in sequence
    // When: we record write-code (pass), format (fail then pass), lint (pass)
    let flow =
        record_step_pass(flow, &write_code, "implement", None, None, None);
    let flow = record_step_fail(
        flow,
        &format_step,
        "implement",
        Some(1),
        Some(ts("2026-04-04T14:10:00Z")),
        Some(300),
    );
    let flow = record_step_pass(
        flow,
        &format_step,
        "implement",
        Some(0),
        Some(ts("2026-04-04T14:11:00Z")),
        Some(280),
    );
    let flow = record_step_pass(
        flow,
        &lint_step,
        "implement",
        Some(0),
        Some(ts("2026-04-04T14:12:00Z")),
        Some(420),
    );

    // Then: four entries are present in definition/call order
    assert_eq!(flow.step_log.len(), 4);
    assert_eq!(flow.step_log[0].step, "write-code");
    assert_eq!(flow.step_log[0].status, StepStatus::Pass);
    assert_eq!(flow.step_log[1].step, "format");
    assert_eq!(flow.step_log[1].status, StepStatus::Fail);
    assert_eq!(flow.step_log[2].step, "format");
    assert_eq!(flow.step_log[2].status, StepStatus::Pass);
    assert_eq!(flow.step_log[3].step, "lint");
    assert_eq!(flow.step_log[3].status, StepStatus::Pass);
}

// ---------------------------------------------------------------------------
// advance preserves accumulated step log
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_preserves_existing_step_log_entries() {
    let workflow = feature_workflow();

    // Given: a flow at design with a completed agent step
    let flow = flow_at("design");
    let write_design = agent_step("write-design");
    let flow =
        record_step_pass(flow, &write_design, "design", None, None, None);
    assert_eq!(flow.step_log.len(), 1);

    // When: we advance the stage
    let advanced =
        advance_stage(&workflow, flow).expect("advance should succeed");

    // Then: the step log entry from the design stage is still present
    assert_eq!(advanced.step_log.len(), 1);
    assert_eq!(advanced.step_log[0].step, "write-design");
    assert_eq!(advanced.step_log[0].stage, "design");
}
