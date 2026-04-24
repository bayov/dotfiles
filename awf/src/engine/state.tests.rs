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
use crate::model::flow::StageStatus;
use crate::model::workflow::LinearKind;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;
use crate::result::Error;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

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
                kind: StageKind::Linear(LinearKind::Agent {
                    prompt_file: Some("design.md".to_owned()),
                    prompt: None,
                }),
            },
            Stage {
                name: "implement".to_owned(),
                description: Some("Write the code.".to_owned()),
                skippable: false,
                produces: vec![],
                kind: StageKind::Linear(LinearKind::Agent {
                    prompt_file: Some("implement.md".to_owned()),
                    prompt: None,
                }),
            },
            Stage {
                name: "format".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Linear(LinearKind::Command {
                    command: Some("format".to_owned()),
                    run: None,
                }),
            },
            Stage {
                name: "lint".to_owned(),
                description: None,
                skippable: false,
                produces: vec![],
                kind: StageKind::Linear(LinearKind::Command {
                    command: Some("lint".to_owned()),
                    run: None,
                }),
            },
            Stage {
                name: "review".to_owned(),
                description: None,
                skippable: false,
                produces: vec!["changelog-entry.md".to_owned()],
                kind: StageKind::Linear(LinearKind::Agent {
                    prompt_file: Some("review.md".to_owned()),
                    prompt: None,
                }),
            },
        ],
        default_flow_dir: None,
    }
}

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
        task_runner_state: HashMap::new(),
        task_log: vec![],
    }
}

fn agent_stage(name: &str) -> Stage {
    Stage {
        name: name.to_owned(),
        description: None,
        skippable: false,
        produces: vec![],
        kind: StageKind::Linear(LinearKind::Agent {
            prompt_file: Some(format!("{name}.md")),
            prompt: None,
        }),
    }
}

fn command_stage(name: &str) -> Stage {
    Stage {
        name: name.to_owned(),
        description: None,
        skippable: false,
        produces: vec![],
        kind: StageKind::Linear(LinearKind::Command {
            run: Some(format!("run-{name}")),
            command: None,
        }),
    }
}

fn ts(s: &str) -> DateTime<Utc> { s.parse().expect("timestamp should parse") }

// ---------------------------------------------------------------------------
// advance_stage — normal advance
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_moves_flow_from_design_to_implement() {
    let workflow = feature_workflow();
    let flow = flow_at("design");

    let updated =
        advance_stage(&workflow, flow, None).expect("advance should succeed");

    assert_eq!(updated.current_stage, "implement");
    assert_eq!(updated.stage_history.len(), 0);
}

#[test]
fn advance_stage_sets_stage_started_at_for_new_stage() {
    let workflow = feature_workflow();
    let flow = flow_at("design");

    let before = Utc::now();
    let updated =
        advance_stage(&workflow, flow, None).expect("advance should succeed");
    let after = Utc::now();

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

    let updated =
        advance_stage(&workflow, flow, None).expect("advance should succeed");

    assert_eq!(updated.updated_at, updated.stage_started_at);
    assert!(
        updated.updated_at >= original_updated_at,
        "updated_at should not move backwards"
    );
}

// ---------------------------------------------------------------------------
// advance_stage — force advance
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_appends_force_completed_entry() {
    let workflow = feature_workflow();
    let flow = flow_at("design");

    let updated =
        advance_stage(&workflow, flow, Some("skipping for now".to_owned()))
            .expect("advance should succeed");

    assert_eq!(updated.current_stage, "implement");
    assert_eq!(updated.stage_history.len(), 1);
    let entry = &updated.stage_history[0];
    assert_eq!(entry.stage, "design");
    assert_eq!(entry.status, StageStatus::ForceCompleted);
    assert_eq!(entry.force_reason.as_deref(), Some("skipping for now"));
    assert!(entry.duration_ms.is_none());
}

// ---------------------------------------------------------------------------
// advance_stage — last stage error
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_returns_already_on_last_stage_when_on_last_stage() {
    let workflow = feature_workflow();
    let flow = flow_at("review");

    let result = advance_stage(&workflow, flow, None);

    assert!(
        matches!(result, Err(Error::AlreadyOnLastStage)),
        "expected Err(AlreadyOnLastStage), got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// record_stage_pass — command stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_pass_appends_passing_command_entry_to_stage_history() {
    let flow = flow_at("format");
    let stage = command_stage("format");

    let before = Utc::now();
    let updated = record_stage_pass(flow, &stage, Some(450));
    let after = Utc::now();

    assert_eq!(updated.stage_history.len(), 1);
    let entry = &updated.stage_history[0];

    assert_eq!(entry.stage, "format");
    assert_eq!(entry.status, StageStatus::Completed);
    assert_eq!(entry.duration_ms, Some(450));
    assert!(entry.force_reason.is_none());
    assert!(
        entry.time >= before && entry.time <= after,
        "time should be within the record_stage_pass call window"
    );
}

// ---------------------------------------------------------------------------
// record_stage_fail — command stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_fail_appends_failing_command_entry_to_stage_history() {
    let flow = flow_at("lint");
    let stage = command_stage("lint");

    let updated = record_stage_fail(flow, &stage, Some(800));

    assert_eq!(updated.stage_history.len(), 1);
    let entry = &updated.stage_history[0];

    assert_eq!(entry.stage, "lint");
    assert_eq!(entry.status, StageStatus::Failed);
    assert_eq!(entry.duration_ms, Some(800));
}

// ---------------------------------------------------------------------------
// record_stage_pass — agent stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_pass_appends_passing_agent_entry_with_time() {
    let flow = flow_at("design");
    let stage = agent_stage("design");

    let before = Utc::now();
    let updated = record_stage_pass(flow, &stage, None);
    let after = Utc::now();

    assert_eq!(updated.stage_history.len(), 1);
    let entry = &updated.stage_history[0];

    assert_eq!(entry.stage, "design");
    assert_eq!(entry.status, StageStatus::Completed);
    assert!(entry.duration_ms.is_none(), "agent stages have no duration_ms");
    assert!(
        entry.time >= before && entry.time <= after,
        "time should be within the record_stage_pass call window"
    );
}

// ---------------------------------------------------------------------------
// record_stage_fail — agent stage
// ---------------------------------------------------------------------------

#[test]
fn record_stage_fail_appends_failing_agent_entry_with_time() {
    let flow = flow_at("design");
    let stage = agent_stage("design");

    let before = Utc::now();
    let updated = record_stage_fail(flow, &stage, None);
    let after = Utc::now();

    assert_eq!(updated.stage_history.len(), 1);
    let entry = &updated.stage_history[0];

    assert_eq!(entry.stage, "design");
    assert_eq!(entry.status, StageStatus::Failed);
    assert!(entry.duration_ms.is_none());
    assert!(
        entry.time >= before && entry.time <= after,
        "time should be within the record_stage_fail call window"
    );
}

// ---------------------------------------------------------------------------
// multiple entries accumulate
// ---------------------------------------------------------------------------

#[test]
fn multiple_record_calls_accumulate_entries_in_stage_history() {
    let flow = flow_at("format");
    let format_stage = command_stage("format");

    let flow = record_stage_fail(flow, &format_stage, Some(300));
    let flow = record_stage_pass(flow, &format_stage, Some(280));

    assert_eq!(flow.stage_history.len(), 2);
    assert_eq!(flow.stage_history[0].stage, "format");
    assert_eq!(flow.stage_history[0].status, StageStatus::Failed);
    assert_eq!(flow.stage_history[1].stage, "format");
    assert_eq!(flow.stage_history[1].status, StageStatus::Completed);
}

// ---------------------------------------------------------------------------
// advance preserves accumulated stage history
// ---------------------------------------------------------------------------

#[test]
fn advance_stage_preserves_existing_stage_history_entries() {
    let workflow = feature_workflow();

    let flow = flow_at("design");
    let design_stage = agent_stage("design");
    let flow = record_stage_pass(flow, &design_stage, None);
    assert_eq!(flow.stage_history.len(), 1);

    let advanced =
        advance_stage(&workflow, flow, None).expect("advance should succeed");

    assert_eq!(advanced.stage_history.len(), 1);
    assert_eq!(advanced.stage_history[0].stage, "design");
    assert_eq!(advanced.stage_history[0].status, StageStatus::Completed);
}

// ---------------------------------------------------------------------------
// skip_stage — skippable stage
// ---------------------------------------------------------------------------

#[test]
fn skip_stage_moves_flow_to_next_stage() {
    let workflow = feature_workflow();
    let flow = flow_at("design");

    let updated = skip_stage(&workflow, flow).expect("skip should succeed");

    assert_eq!(updated.current_stage, "implement");
    assert_eq!(updated.stage_history.len(), 1);
    let entry = &updated.stage_history[0];
    assert_eq!(entry.stage, "design");
    assert_eq!(entry.status, StageStatus::Skipped);
    assert!(entry.force_reason.is_none());
    assert!(entry.duration_ms.is_none());
}

#[test]
fn skip_stage_errors_on_non_skippable_stage() {
    let workflow = feature_workflow();
    let flow = flow_at("implement");

    let result = skip_stage(&workflow, flow);

    assert!(
        matches!(result, Err(Error::StageNotSkippable { .. })),
        "expected Err(StageNotSkippable), got {result:?}"
    );
}

#[test]
fn skip_stage_errors_on_last_stage() {
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
            kind: StageKind::Linear(LinearKind::Agent {
                prompt_file: Some("do.md".to_owned()),
                prompt: None,
            }),
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
