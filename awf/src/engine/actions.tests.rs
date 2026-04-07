//! Tests for [`super::next_action`] and [`super::can_advance`].
//!
//! All tests use the canonical "feature" workflow fixture with stages:
//! `design` → `implement` → `format` → `lint` → `review`.

use std::collections::HashMap;
use std::collections::HashSet;

use chrono::DateTime;
use chrono::Utc;

use super::AdvanceBlocker;
use super::NextAction;
use super::can_advance;
use super::next_action;
use crate::model::flow::FlowState;
use crate::model::flow::StageHistoryEntry;
use crate::model::flow::StageStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Builds the canonical "feature" workflow used across all tests.
///
/// Stages:
/// - `design`: agent stage; produces `design.md`
/// - `implement`: agent stage
/// - `format`: command stage
/// - `lint`: command stage
/// - `review`: agent stage; produces `changelog-entry.md`
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
                skippable: false,
                produces: vec!["design.md".to_owned()],
                kind: StageKind::Agent {
                    prompt: Some("design.md".to_owned()),
                    instructions: None,
                },
            },
            Stage {
                name: "implement".to_owned(),
                description: Some("Write the code.".to_owned()),
                skippable: false,
                produces: vec![],
                kind: StageKind::Agent {
                    prompt: Some("implement.md".to_owned()),
                    instructions: None,
                },
            },
            Stage {
                name: "format".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Command {
                    command: Some("format".to_owned()),
                    run: None,
                },
            },
            Stage {
                name: "lint".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Command {
                    command: Some("lint".to_owned()),
                    run: None,
                },
            },
            Stage {
                name: "review".to_owned(),
                description: None,
                skippable: false,
                produces: vec!["changelog-entry.md".to_owned()],
                kind: StageKind::Agent {
                    prompt: Some("review.md".to_owned()),
                    instructions: None,
                },
            },
        ],
        default_flow_dir: None,
    }
}

/// Builds a minimal [`FlowState`] for the "feature" workflow at the given
/// stage with the provided stage history entries.
fn flow_at(stage: &str, stage_history: Vec<StageHistoryEntry>) -> FlowState {
    FlowState {
        id: "add-oauth".to_owned(),
        workflow: "feature".to_owned(),
        workflow_version: 1,
        current_stage: stage.to_owned(),
        stage_started_at: ts("2026-04-04T10:00:00Z"),
        created_at: ts("2026-04-04T10:00:00Z"),
        updated_at: ts("2026-04-04T10:00:00Z"),
        archived: false,
        stage_history,
    }
}

/// Builds a [`StageStatus::Completed`] entry for a command stage.
fn pass_cmd(stage: &str) -> StageHistoryEntry {
    StageHistoryEntry {
        stage: stage.to_owned(),
        time: ts("2026-04-04T14:00:00Z"),
        status: StageStatus::Completed,
        force_reason: None,
        duration_ms: Some(500),
    }
}

/// Builds a [`StageStatus::Failed`] entry for a command stage.
fn fail_cmd(stage: &str) -> StageHistoryEntry {
    StageHistoryEntry {
        stage: stage.to_owned(),
        time: ts("2026-04-04T14:00:00Z"),
        status: StageStatus::Failed,
        force_reason: None,
        duration_ms: Some(800),
    }
}

/// Builds a [`StageStatus::Completed`] entry for an agent stage.
fn pass_agent(stage: &str) -> StageHistoryEntry {
    StageHistoryEntry {
        stage: stage.to_owned(),
        time: ts("2026-04-04T13:00:00Z"),
        status: StageStatus::Completed,
        force_reason: None,
        duration_ms: None,
    }
}

/// Parse an RFC 3339 timestamp into [`DateTime<Utc>`], panicking on failure.
fn ts(s: &str) -> DateTime<Utc> { s.parse().expect("timestamp should parse") }

// ---------------------------------------------------------------------------
// next_action — NextAction::RunCommand (failing command stage, priority 1)
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_run_command_for_failing_command_stage() {
    let workflow = feature_workflow();

    // Given: the format stage, where `format` failed
    let flow = flow_at("format", vec![fail_cmd("format")]);

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &HashSet::new());

    // Then: the failing command stage is returned
    let format_stage = &workflow.stages[2]; // "format"
    assert_eq!(action, NextAction::RunCommand(format_stage));
}

// ---------------------------------------------------------------------------
// next_action — NextAction::RunCommand (pending command stage, priority 2)
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_run_command_for_pending_command_stage() {
    let workflow = feature_workflow();

    // Given: the format stage with no history entries
    let flow = flow_at("format", vec![]);

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &HashSet::new());

    // Then: the command stage is returned
    let format_stage = &workflow.stages[2]; // "format"
    assert_eq!(action, NextAction::RunCommand(format_stage));
}

// ---------------------------------------------------------------------------
// next_action — NextAction::CompleteAgent
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_complete_agent_for_pending_agent_stage() {
    let workflow = feature_workflow();

    // Given: the implement stage with no history entries
    let flow = flow_at("implement", vec![]);

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &HashSet::new());

    // Then: the agent stage is returned
    let implement_stage = &workflow.stages[1]; // "implement"
    assert_eq!(action, NextAction::CompleteAgent(implement_stage));
}

// ---------------------------------------------------------------------------
// next_action — NextAction::ProduceArtifact
// ---------------------------------------------------------------------------

#[test]
fn next_action_returns_produce_artifact_when_artifact_missing() {
    let workflow = feature_workflow();

    // Given: the design stage with the agent stage completed, but the
    // `design.md` artifact not yet registered
    let flow = flow_at("design", vec![pass_agent("design")]);
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

    // Given: the design stage is fully done (stage passed, artifact present),
    // and this is not the last stage
    let flow = flow_at("design", vec![pass_agent("design")]);
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

    // Given: the review stage (last stage) is fully done — stage passed
    // and the `changelog-entry.md` artifact is present
    let flow = flow_at("review", vec![pass_agent("review")]);
    let mut artifacts = HashSet::new();
    artifacts.insert("changelog-entry.md".to_owned());

    // When: we ask for the next action
    let action = next_action(&workflow, &flow, &artifacts);

    // Then: the workflow is done — nothing more to do
    assert_eq!(action, NextAction::Done);
}

// ---------------------------------------------------------------------------
// can_advance — StageFailed blocker
// ---------------------------------------------------------------------------

#[test]
fn can_advance_returns_stage_failed_when_last_entry_is_fail() {
    let workflow = feature_workflow();

    // Given: the format stage where `format` failed
    let flow = flow_at("format", vec![fail_cmd("format")]);

    // When: we check whether the stage can advance
    let result = can_advance(&workflow, &flow, &HashSet::new());

    // Then: blocked because the stage has failed
    assert_eq!(result, Err(AdvanceBlocker::StageFailed("format".to_owned())));
}

// ---------------------------------------------------------------------------
// can_advance — StageIncomplete blocker
// ---------------------------------------------------------------------------

#[test]
fn can_advance_returns_stage_incomplete_when_stage_has_no_pass_entry() {
    let workflow = feature_workflow();

    // Given: the format stage with no history entries
    let flow = flow_at("format", vec![]);

    // When: we check whether the stage can advance
    let result = can_advance(&workflow, &flow, &HashSet::new());

    // Then: blocked because the stage is incomplete
    assert_eq!(
        result,
        Err(AdvanceBlocker::StageIncomplete("format".to_owned()))
    );
}

// ---------------------------------------------------------------------------
// can_advance — MissingArtifact blocker
// ---------------------------------------------------------------------------

#[test]
fn can_advance_returns_missing_artifact_when_produces_artifact_is_absent() {
    let workflow = feature_workflow();

    // Given: the design stage where the stage passed, but the produced
    // artifact has not been registered
    let flow = flow_at("design", vec![pass_agent("design")]);
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
fn can_advance_returns_ok_when_stage_passes_and_artifacts_present() {
    let workflow = feature_workflow();

    // Given: the design stage where the stage passed and the artifact is
    // registered
    let flow = flow_at("design", vec![pass_agent("design")]);
    let mut artifacts = HashSet::new();
    artifacts.insert("design.md".to_owned());

    // When: we check whether the stage can advance
    let result = can_advance(&workflow, &flow, &artifacts);

    // Then: no blockers — the stage may advance
    assert_eq!(result, Ok(()));
}

// ---------------------------------------------------------------------------
// can_advance — stage passes after earlier failure (re-run scenario)
// ---------------------------------------------------------------------------

#[test]
fn can_advance_passes_when_stage_reruns_and_last_entry_is_pass() {
    let workflow = feature_workflow();

    // Given: the format stage where `format` initially failed, was re-run,
    // and then passed
    let flow = flow_at("format", vec![
        fail_cmd("format"), // first attempt: fail
        pass_cmd("format"), // second attempt: pass
    ]);

    // When: we check whether the stage can advance (format has no produces)
    let result = can_advance(&workflow, &flow, &HashSet::new());

    // Then: the stage can advance — the last entry is a pass
    assert_eq!(result, Ok(()));
}
