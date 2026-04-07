//! Stage-transition and stage-recording logic for a flow in progress.
//!
//! The public entry points are:
//!
//! - [`advance_stage`] — advances the flow to the next stage in the workflow.
//!   For a force-advance, appends a [`StageHistoryEntry`] with
//!   [`StageStatus::ForceCompleted`] before moving forward. For a normal
//!   advance, the passing entry was already appended by [`record_stage_pass`].
//!
//! - [`record_stage_pass`] / [`record_stage_fail`] — append a
//!   [`StageHistoryEntry`] to the flow's stage history recording the outcome of
//!   a completed stage run.

use chrono::Utc;
use tracing::instrument;

use crate::model::flow::FlowState;
use crate::model::flow::StageHistoryEntry;
use crate::model::flow::StageStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::result::Result;

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Advances the flow to the next stage in the workflow.
///
/// For a force-advance (`force_reason` is `Some`), appends a
/// [`StageHistoryEntry`] with [`StageStatus::ForceCompleted`] and the given
/// reason before moving forward. For a normal advance, the
/// [`StageStatus::Completed`] entry was already appended by
/// [`record_stage_pass`] and no additional entry is written.
///
/// Sets `current_stage` and `stage_started_at` to the next stage and updates
/// `updated_at`.
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

    // For a force-advance, record the transition event. For a normal advance
    // the completed entry was already appended by record_stage_pass.
    if let Some(reason) = force_reason {
        flow.stage_history.push(StageHistoryEntry {
            stage: flow.current_stage.clone(),
            time: now,
            status: StageStatus::ForceCompleted,
            force_reason: Some(reason),
            duration_ms: None,
        });
    }

    // Advance to the next stage and refresh timestamps.
    flow.current_stage = next_stage_name;
    flow.stage_started_at = now;
    flow.updated_at = now;

    Ok(flow)
}

/// Skips the current stage, recording it as skipped in the history.
///
/// Validates that the current stage is marked `skippable` in the workflow.
/// Bypasses all gates (stage completion and artifacts).
///
/// # Errors
///
/// Returns [`Error::StageNotSkippable`] when the current stage is not marked
/// as `skippable`. Returns [`Error::AlreadyOnLastStage`] when the flow is
/// already on the last stage.
#[instrument(skip_all, fields(
    workflow.name = %workflow.name,
    flow.id = %flow.id,
    flow.current_stage = %flow.current_stage,
))]
pub fn skip_stage(
    workflow: &Workflow,
    mut flow: FlowState,
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

    // Validate the stage is marked as skippable.
    if !workflow.stages[current_idx].skippable {
        return Err(Error::StageNotSkippable {
            stage: flow.current_stage.clone(),
        });
    }

    // Fail fast if already on the last stage — there is nowhere to skip to.
    if current_idx + 1 >= workflow.stages.len() {
        return Err(Error::AlreadyOnLastStage);
    }

    let next_stage_name = workflow.stages[current_idx + 1].name.clone();
    let now = Utc::now();

    // Record the skipped stage in history.
    flow.stage_history.push(StageHistoryEntry {
        stage: flow.current_stage.clone(),
        time: now,
        status: StageStatus::Skipped,
        force_reason: None,
        duration_ms: None,
    });

    // Advance to the next stage and refresh timestamps.
    flow.current_stage = next_stage_name;
    flow.stage_started_at = now;
    flow.updated_at = now;

    Ok(flow)
}

/// Appends a [`StageStatus::Completed`] entry for `stage` to the flow's
/// stage history.
///
/// For command stages, `duration_ms` is recorded. For agent stages,
/// `duration_ms` should be `None`.
///
/// Returns the updated [`FlowState`].
pub fn record_stage_pass(
    mut flow: FlowState,
    stage: &Stage,
    duration_ms: Option<u64>,
) -> FlowState {
    flow.stage_history.push(StageHistoryEntry {
        stage: stage.name.clone(),
        time: Utc::now(),
        status: StageStatus::Completed,
        force_reason: None,
        duration_ms: if matches!(stage.kind, StageKind::Command { .. }) {
            duration_ms
        } else {
            None
        },
    });
    flow.updated_at = Utc::now();
    flow
}

/// Appends a [`StageStatus::Failed`] entry for `stage` to the flow's stage
/// history.
///
/// For command stages, `duration_ms` is recorded. For agent stages,
/// `duration_ms` should be `None`.
///
/// Returns the updated [`FlowState`].
pub fn record_stage_fail(
    mut flow: FlowState,
    stage: &Stage,
    duration_ms: Option<u64>,
) -> FlowState {
    flow.stage_history.push(StageHistoryEntry {
        stage: stage.name.clone(),
        time: Utc::now(),
        status: StageStatus::Failed,
        force_reason: None,
        duration_ms: if matches!(stage.kind, StageKind::Command { .. }) {
            duration_ms
        } else {
            None
        },
    });
    flow.updated_at = Utc::now();
    flow
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "state.tests.rs"]
mod tests;
