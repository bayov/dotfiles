//! Stage-transition and step-recording logic for a flow in progress.
//!
//! The public entry points are:
//!
//! - [`advance_stage`] — advances the flow to the next stage in the workflow,
//!   appending a completed [`StageHistoryEntry`] for the stage being left.
//!
//! - [`record_step_pass`] / [`record_step_fail`] — append a [`StepLogEntry`] to
//!   the flow's step log recording the outcome of a completed step.

use chrono::Utc;
use tracing::instrument;

use crate::model::flow::FlowState;
use crate::model::flow::StageHistoryEntry;
use crate::model::flow::StepLogEntry;
use crate::model::flow::StepStatus;
use crate::model::workflow::Step;
use crate::model::workflow::StepKind;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::result::Result;

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Advances the flow to the next stage in the workflow.
///
/// Appends a completed [`StageHistoryEntry`] for the stage being left (using
/// `flow.stage_started_at` as `started_at` and `Utc::now()` as `completed_at`),
/// sets `current_stage` and `stage_started_at` to the next stage, and updates
/// `updated_at`. When `force_reason` is `Some`, the history entry is marked
/// with `force_completed = true` and the reason is stored.
///
/// # Errors
///
/// Returns [`Error::AlreadyOnLastStage`] when the flow is already on the last
/// stage of the workflow.
///
/// # Panics
///
/// Panics if `flow.current_stage` does not name a stage in `workflow.stages`.
#[instrument(skip_all, fields(
    workflow.name = %workflow.name,
    flow.id = %flow.id,
    flow.current_stage = %flow.current_stage,
))]
pub fn advance_stage(
    workflow: &Workflow,
    mut flow: FlowState,
    force_reason: Option<String>,
) -> Result<FlowState> {
    let current_idx = workflow
        .stages
        .iter()
        .position(|s| s.name == flow.current_stage)
        .unwrap_or_else(|| {
            panic!(
                "current stage {:?} not found in workflow {:?}",
                flow.current_stage, workflow.name
            )
        });

    // Fail fast if already on the last stage — there is nowhere to advance.
    if current_idx + 1 >= workflow.stages.len() {
        return Err(Error::AlreadyOnLastStage);
    }

    let next_stage_name = workflow.stages[current_idx + 1].name.clone();
    let now = Utc::now();

    // Record the completed stage in history before moving forward.
    let force_completed = force_reason.is_some();
    flow.stage_history.push(StageHistoryEntry {
        stage: flow.current_stage.clone(),
        started_at: flow.stage_started_at,
        completed_at: Some(now),
        force_completed,
        force_reason,
    });

    // Advance to the next stage and refresh timestamps.
    flow.current_stage = next_stage_name;
    flow.stage_started_at = now;
    flow.updated_at = now;

    Ok(flow)
}

/// Appends a passing [`StepLogEntry`] for `step` to the flow's step log.
///
/// For command steps, `exit_code`, `started_at`, and `duration_ms` are
/// recorded. For agent steps, `completed_at` is set to `Utc::now()` and the
/// command-specific fields are left as `None`.
///
/// Returns the updated [`FlowState`].
pub fn record_step_pass(
    mut flow: FlowState,
    step: &Step,
    stage: &str,
    exit_code: Option<i32>,
    started_at: Option<chrono::DateTime<Utc>>,
    duration_ms: Option<u64>,
) -> FlowState {
    flow.step_log.push(build_log_entry(
        step,
        stage,
        StepStatus::Pass,
        exit_code,
        started_at,
        duration_ms,
    ));
    flow.updated_at = Utc::now();
    flow
}

/// Appends a failing [`StepLogEntry`] for `step` to the flow's step log.
///
/// For command steps, `exit_code`, `started_at`, and `duration_ms` are
/// recorded. For agent steps, `completed_at` is set to `Utc::now()` and the
/// command-specific fields are left as `None`.
///
/// Returns the updated [`FlowState`].
pub fn record_step_fail(
    mut flow: FlowState,
    step: &Step,
    stage: &str,
    exit_code: Option<i32>,
    started_at: Option<chrono::DateTime<Utc>>,
    duration_ms: Option<u64>,
) -> FlowState {
    flow.step_log.push(build_log_entry(
        step,
        stage,
        StepStatus::Fail,
        exit_code,
        started_at,
        duration_ms,
    ));
    flow.updated_at = Utc::now();
    flow
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Constructs a [`StepLogEntry`] from a step's metadata and execution outcome.
///
/// The `step_type` string is derived from the step's [`StepKind`] variant.
/// Agent steps populate `completed_at = Some(Utc::now())` and leave command
/// fields (`exit_code`, `started_at`, `duration_ms`) as `None`. Command steps
/// record the provided execution metrics and leave `completed_at` as `None`.
fn build_log_entry(
    step: &Step,
    stage: &str,
    status: StepStatus,
    exit_code: Option<i32>,
    started_at: Option<chrono::DateTime<Utc>>,
    duration_ms: Option<u64>,
) -> StepLogEntry {
    match &step.kind {
        StepKind::Agent { .. } => StepLogEntry {
            step: step.name.clone(),
            stage: stage.to_owned(),
            step_type: "agent".to_owned(),
            status,
            exit_code: None,
            started_at: None,
            duration_ms: None,
            completed_at: Some(Utc::now()),
        },
        StepKind::Command { .. } => StepLogEntry {
            step: step.name.clone(),
            stage: stage.to_owned(),
            step_type: "command".to_owned(),
            status,
            exit_code,
            completed_at: started_at.zip(duration_ms).map(|(s, d)| {
                s + chrono::Duration::milliseconds(d.cast_signed())
            }),
            started_at,
            duration_ms,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "state.tests.rs"]
mod tests;
