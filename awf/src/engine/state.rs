//! Stage-transition and step-recording logic for a flow in progress.
//!
//! The three public entry points are:
//!
//! - [`can_enter_stage`] — given a workflow, flow state, target stage name, and
//!   the set of already-produced artifacts, returns an [`EnterResult`]
//!   describing whether the stage may be entered.
//!
//! - [`advance_stage`] — advances the flow to the next stage in the workflow,
//!   appending a completed [`StageHistoryEntry`] for the stage being left.
//!
//! - [`record_step_pass`] / [`record_step_fail`] — append a [`StepLogEntry`] to
//!   the flow's step log recording the outcome of a completed step.
//!
//! All functions except [`advance_stage`] are pure (no I/O).

use std::collections::HashSet;

use chrono::Utc;
use tracing::instrument;

use crate::model::flow::FlowState;
use crate::model::flow::StageHistoryEntry;
use crate::model::flow::StepLogEntry;
use crate::model::flow::StepStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::Step;
use crate::model::workflow::StepKind;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::result::Result;

// ---------------------------------------------------------------------------
// EnterResult
// ---------------------------------------------------------------------------

/// The outcome of checking whether a stage may be entered.
///
/// Returned by [`can_enter_stage`].
#[derive(Debug, PartialEq, Eq)]
pub enum EnterResult {
    /// The stage may be entered — all requirements are satisfied.
    Ok,

    /// One or more required artifacts are absent from the artifact set.
    ///
    /// The inner `Vec` lists each missing artifact name.
    MissingRequires(Vec<String>),

    /// The stage has already been completed and is not marked `repeatable`.
    ///
    /// The inner `produces` field lists the artifact names declared by the
    /// stage's `produces` list. The CLI layer can use these names to read
    /// and display the already-produced artifacts.
    AlreadyCompleted { produces: Vec<String> },
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Checks whether the named stage may be entered given the current flow state
/// and artifact set.
///
/// The check proceeds in priority order:
///
/// 1. If any artifact in the stage's `requires` list is absent from
///    `artifacts`, returns [`EnterResult::MissingRequires`] with the names of
///    all missing artifacts.
/// 2. If the stage already appears in `flow.stage_history` **and** the stage's
///    `repeatable` flag is `false`, returns [`EnterResult::AlreadyCompleted`]
///    with the stage's `produces` list.
/// 3. Otherwise returns [`EnterResult::Ok`].
///
/// # Panics
///
/// Panics if `stage_name` does not name a stage in `workflow.stages`.
#[instrument(skip_all, fields(
    workflow.name = %workflow.name,
    flow.id = %flow.id,
    stage_name = stage_name,
))]
pub fn can_enter_stage(
    workflow: &Workflow,
    flow: &FlowState,
    stage_name: &str,
    artifacts: &HashSet<String>,
) -> EnterResult {
    let stage = find_stage(workflow, stage_name);

    // Check 1: all required artifacts must be present.
    let missing: Vec<String> = stage
        .requires
        .iter()
        .filter(|req| !artifacts.contains(*req))
        .cloned()
        .collect();

    if !missing.is_empty() {
        return EnterResult::MissingRequires(missing);
    }

    // Check 2: if non-repeatable, the stage must not already be in history.
    if !stage.repeatable
        && flow.stage_history.iter().any(|e| e.stage == stage_name)
    {
        return EnterResult::AlreadyCompleted {
            produces: stage.produces.clone(),
        };
    }

    EnterResult::Ok
}

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

/// Finds the [`Stage`] in `workflow` with the given `name`.
///
/// # Panics
///
/// Panics if no stage with that name exists.
fn find_stage<'a>(workflow: &'a Workflow, name: &str) -> &'a Stage {
    workflow.stages.iter().find(|s| s.name == name).unwrap_or_else(|| {
        panic!("stage {name:?} not found in workflow {:?}", workflow.name)
    })
}

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
            started_at,
            duration_ms,
            completed_at: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "state.tests.rs"]
mod tests;
