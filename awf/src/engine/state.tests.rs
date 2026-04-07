//! Tests for [`super::advance_stage`], [`super::record_stage_pass`], and
//! [`super::record_stage_fail`].
//!
//! All tests use the canonical "feature" workflow fixture with stages:
//! `design` → `implement` → `format` → `lint` → `review`.

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;

use super::advance_stage;
use super::record_stage_fail;
use super::record_stage_pass;
use super::skip_stage;
use crate::model::flow::FlowState;
use crate::model::flow::RunStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;
use crate::result::Error;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Builds the canonical "feature" workflow used across all tests.
///
/// Stages:
/// - `design` (produces `design.md`): agent stage
/// - `implement`: agent stage
/// - `format`: command stage
/// - `lint`: command stage
/// - `review` (produces `changelog-entry.md`): agent stage
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
                skippable: true,
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
/// stage with no stage log and no stage history.
fn flow_at(stage: &str) -> FlowState {
    FlowState {
        id: "add-oauth".to_owned(),
        workflow: "feature".to_owned(),
        workflow_version: 1,
        current_stage: stage.to_owned(),
        stage_started_at: ts("2026-04-04T10:00:00Z"),
        created_at: ts("2026-04-04T10:00:00Z"),
        updated_at: ts("2026-04-04T10:00:00Z"),
        archived: false,
        stage_history: vec![],
        stage_log: vec![],
    }
}

/// Builds a minimal agent [`Stage`].
fn agent_stage(name: &str) -> Stage {
    Stage {
        name: name.to_owned(),
        description: None,
        skippable: false,
        produces: vec![],
        kind: StageKind::Agent {
            prompt: Some(format!("{name}.md")),
            instructions: None,
        },
    }
}

/// Builds a minimal command [`Stage`].
fn command_stage(name: &str) -> Stage {
    Stage {
        name: name.to_owned(),
        description: None,
        skippable: false,
        produces: vec![],
        kind: StageKind::Command {
            run: Some(format!("run-{name}")),
            command: None,
        },
    }
}

/// Parse an RFC 3339 timestamp into [`DateTime<Utc>`], panicking on failure.
fn ts(s: &str) -> DateTime<Utc> { s.parse().expect("timestamp should parse") }

// ---------------------------------------------------------------------------
// advance_stage — normal advance
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_moves_flow_from_design_to_implement() {
    let workflow = feature_workflow();

    // Given: a flow at the design stage, entered at a known time
    let mut flow = flow_at("design");
    flow.stage_started_at = ts("2026-04-04T10:00:00Z");

    // When: we advance the stage
    let updated =
        advance_stage(&workflow, flow, None).expect("advance should succeed");

    // Then: the current stage is now implement
    assert_eq!(updated.current_stage, "implement");

    // Then: the stage history records the completed design stage
    assert_eq!(updated.stage_history.len(), 1);
    let history_entry = &updated.stage_history[0];
    assert_eq!(history_entry.stage, "design");
    assert_eq!(history_entry.started_at, ts("2026-04-04T10:00:00Z"));
    assert!(
        history_entry.completed_at.is_some(),
        "completed_at should be set after advancing"
    );
}

#[test]
fn advance_stage_sets_stage_started_at_for_new_stage() {
    let workflow = feature_workflow();
    let flow = flow_at("design");

    // When: we advance the stage
    let before = Utc::now();
    let updated =
        advance_stage(&workflow, flow, None).expect("advance should succeed");
    let after = Utc::now();

    // Then: stage_started_at is set to a time within the current call window
    assert!(
        updated.stage_started_at >= before && updated.stage_started_at <= after,
        "stage_started_at should be within the advance_stage call window"
    );
}

#[test]
fn advance_stage_updates_updated_at() {
    let workflow = feature_workflow();
    let flow = flow_at("design");
    let original_updated_at = flow.updated_at;

    // When: we advance the stage
    let updated =
        advance_stage(&workflow, flow, None).expect("advance should succeed");

    // Then: updated_at is refreshed (same instant as stage_started_at)
    assert_eq!(updated.updated_at, updated.stage_started_at);
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
    let result = advance_stage(&workflow, flow, None);

    // Then: AlreadyOnLastStage error is returned
    assert!(
        matches!(result, Err(Error::AlreadyOnLastStage)),
        "expected Err(AlreadyOnLastStage), got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// record_stage_pass — command stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_pass_appends_passing_command_entry_to_stage_log() {
    let flow = flow_at("format");
    let stage = command_stage("format");

    // Given: the format stage flow and a command stage
    // When: we record a passing run (exit code 0, 450 ms)
    let updated = record_stage_pass(
        flow,
        &stage,
        Some(0),
        Some(ts("2026-04-04T14:00:00Z")),
        Some(450),
    );

    // Then: one entry is in the stage log
    assert_eq!(updated.stage_log.len(), 1);
    let entry = &updated.stage_log[0];

    assert_eq!(entry.stage, "format");
    assert_eq!(entry.stage_kind, "command");
    assert_eq!(entry.status, RunStatus::Pass);
    assert_eq!(entry.exit_code, Some(0));
    assert_eq!(entry.started_at, Some(ts("2026-04-04T14:00:00Z")));
    assert_eq!(entry.duration_ms, Some(450));
    assert_eq!(
        entry.completed_at,
        Some(ts("2026-04-04T14:00:00.450Z")),
        "completed_at should be started_at + duration_ms"
    );
}

// ---------------------------------------------------------------------------
// record_stage_fail — command stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_fail_appends_failing_command_entry_to_stage_log() {
    let flow = flow_at("lint");
    let stage = command_stage("lint");

    // Given: the lint stage flow and a command stage
    // When: we record a failing run (exit code 1, 800 ms)
    let updated = record_stage_fail(
        flow,
        &stage,
        Some(1),
        Some(ts("2026-04-04T14:05:00Z")),
        Some(800),
    );

    // Then: one failing entry is in the stage log
    assert_eq!(updated.stage_log.len(), 1);
    let entry = &updated.stage_log[0];

    assert_eq!(entry.stage, "lint");
    assert_eq!(entry.stage_kind, "command");
    assert_eq!(entry.status, RunStatus::Fail);
    assert_eq!(entry.exit_code, Some(1));
    assert_eq!(entry.started_at, Some(ts("2026-04-04T14:05:00Z")));
    assert_eq!(entry.duration_ms, Some(800));
    assert_eq!(
        entry.completed_at,
        Some(ts("2026-04-04T14:05:00.800Z")),
        "completed_at should be started_at + duration_ms"
    );
}

// ---------------------------------------------------------------------------
// record_stage_pass — agent stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_pass_appends_passing_agent_entry_with_completed_at() {
    let flow = flow_at("design");
    let stage = agent_stage("design");

    // Given: the design stage flow and an agent stage
    // When: we record a passing agent completion (command fields are None)
    let before = Utc::now();
    let updated = record_stage_pass(flow, &stage, None, None, None);
    let after = Utc::now();

    // Then: one passing agent entry is in the stage log
    assert_eq!(updated.stage_log.len(), 1);
    let entry = &updated.stage_log[0];

    assert_eq!(entry.stage, "design");
    assert_eq!(entry.stage_kind, "agent");
    assert_eq!(entry.status, RunStatus::Pass);
    assert!(entry.exit_code.is_none());
    assert!(entry.started_at.is_none());
    assert!(entry.duration_ms.is_none());

    // Then: completed_at is set to a time within the record_stage_pass call
    let completed_at =
        entry.completed_at.expect("agent stage should have completed_at set");
    assert!(
        completed_at >= before && completed_at <= after,
        "completed_at should be within the record_stage_pass call window"
    );
}

// ---------------------------------------------------------------------------
// record_stage_fail — agent stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_fail_appends_failing_agent_entry_with_completed_at() {
    let flow = flow_at("design");
    let stage = agent_stage("design");

    // Given: the design stage flow and an agent stage
    // When: we record a failing agent completion
    let before = Utc::now();
    let updated = record_stage_fail(flow, &stage, None, None, None);
    let after = Utc::now();

    // Then: one failing agent entry is in the stage log
    assert_eq!(updated.stage_log.len(), 1);
    let entry = &updated.stage_log[0];

    assert_eq!(entry.stage, "design");
    assert_eq!(entry.stage_kind, "agent");
    assert_eq!(entry.status, RunStatus::Fail);
    assert!(entry.exit_code.is_none());

    // Then: completed_at is set
    let completed_at =
        entry.completed_at.expect("agent stage should have completed_at set");
    assert!(
        completed_at >= before && completed_at <= after,
        "completed_at should be within the record_stage_fail call window"
    );
}

// ---------------------------------------------------------------------------
// multiple entries accumulate in stage_log
// ---------------------------------------------------------------------------

#[test]
fn multiple_record_calls_accumulate_entries_in_stage_log() {
    let flow = flow_at("format");
    let format_stage = command_stage("format");

    // Given: the format stage being run multiple times
    // When: we record fail then pass
    let flow = record_stage_fail(
        flow,
        &format_stage,
        Some(1),
        Some(ts("2026-04-04T14:10:00Z")),
        Some(300),
    );
    let flow = record_stage_pass(
        flow,
        &format_stage,
        Some(0),
        Some(ts("2026-04-04T14:11:00Z")),
        Some(280),
    );

    // Then: two entries are present in call order
    assert_eq!(flow.stage_log.len(), 2);
    assert_eq!(flow.stage_log[0].stage, "format");
    assert_eq!(flow.stage_log[0].status, RunStatus::Fail);
    assert_eq!(flow.stage_log[1].stage, "format");
    assert_eq!(flow.stage_log[1].status, RunStatus::Pass);
}

// ---------------------------------------------------------------------------
// advance preserves accumulated stage log
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_preserves_existing_stage_log_entries() {
    let workflow = feature_workflow();

    // Given: a flow at design with a completed agent stage
    let flow = flow_at("design");
    let design_stage = agent_stage("design");
    let flow = record_stage_pass(flow, &design_stage, None, None, None);
    assert_eq!(flow.stage_log.len(), 1);

    // When: we advance the stage
    let advanced =
        advance_stage(&workflow, flow, None).expect("advance should succeed");

    // Then: the stage log entry from the design stage is still present
    assert_eq!(advanced.stage_log.len(), 1);
    assert_eq!(advanced.stage_log[0].stage, "design");
}

// ---------------------------------------------------------------------------
// skip_stage — skippable stage
// ---------------------------------------------------------------------------

#[test]
fn skip_stage_moves_flow_to_next_stage() {
    let workflow = feature_workflow();

    // Given: a flow at the design stage (which is skippable)
    let flow = flow_at("design");

    // When: we skip the stage
    let updated = skip_stage(&workflow, flow).expect("skip should succeed");

    // Then: the current stage is now implement
    assert_eq!(updated.current_stage, "implement");

    // Then: the stage history records the skipped design stage
    assert_eq!(updated.stage_history.len(), 1);
    let history_entry = &updated.stage_history[0];
    assert_eq!(history_entry.stage, "design");
    assert!(history_entry.skipped, "stage should be marked as skipped");
    assert!(!history_entry.force_completed);
    assert!(history_entry.completed_at.is_some());
}

#[test]
fn skip_stage_errors_on_non_skippable_stage() {
    let workflow = feature_workflow();

    // Given: a flow at the implement stage (which is NOT skippable)
    let flow = flow_at("implement");

    // When: we attempt to skip
    let result = skip_stage(&workflow, flow);

    // Then: StageNotSkippable error is returned
    assert!(
        matches!(result, Err(Error::StageNotSkippable { .. })),
        "expected Err(StageNotSkippable), got {result:?}"
    );
}

#[test]
fn skip_stage_errors_on_last_stage() {
    // Build a workflow where the last stage is skippable
    let workflow = Workflow {
        name: "single".to_owned(),
        version: 1,
        description: None,
        commands: HashMap::new(),
        stages: vec![Stage {
            name: "only".to_owned(),
            description: None,
            skippable: true,
            produces: vec![],
            kind: StageKind::Agent {
                prompt: Some("do.md".to_owned()),
                instructions: None,
            },
        }],
        default_flow_dir: None,
    };

    let flow = flow_at("only");
    let result = skip_stage(&workflow, flow);

    assert!(
        matches!(result, Err(Error::AlreadyOnLastStage)),
        "expected Err(AlreadyOnLastStage), got {result:?}"
    );
}
