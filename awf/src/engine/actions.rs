//! Next-action and advance-blocker evaluation for a flow in progress.
//!
//! The two public entry points are:
//!
//! - [`next_action`] — given the current workflow, flow state, and the set of
//!   already-produced artifacts, returns the single highest-priority action the
//!   user (or tool) should take next.
//!
//! - [`can_advance`] — answers whether the current stage is complete and the
//!   flow may advance to its next stage; returns the first blocking reason
//!   found if not.
//!
//! Both functions are pure (no I/O) and operate only on references.

use std::collections::HashSet;

use tracing::instrument;

use crate::model::flow::FlowState;
use crate::model::flow::StageStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;

// ---------------------------------------------------------------------------
// NextAction
// ---------------------------------------------------------------------------

/// The highest-priority action to take next in the current flow.
///
/// Variants are listed in priority order — [`RunCommand`] for a failing
/// command stage has the highest urgency, while [`Done`] signals that the
/// entire workflow is complete.
///
/// [`RunCommand`]: NextAction::RunCommand
/// [`Done`]: NextAction::Done
#[derive(Debug, PartialEq, Eq)]
pub enum NextAction<'a> {
    /// Run (or re-run) this command stage.
    ///
    /// Returned when the current stage is a command kind and its most recent
    /// log entry is `fail` or it has not yet been run at all.
    RunCommand(&'a Stage),

    /// Complete this agent stage.
    ///
    /// Returned when the current stage is an agent kind and has not yet
    /// received a `pass` log entry.
    CompleteAgent(&'a Stage),

    /// Register the named artifact before advancing.
    ///
    /// Returned when the current stage has passed but one of its declared
    /// `produces` artifacts is absent from the artifact set.
    ProduceArtifact(String),

    /// The stage is complete and all required artifacts are present — advance.
    ///
    /// Returned when the current stage has a `pass` log entry, every
    /// `produces` artifact is in the artifact set, and the flow is **not** on
    /// the last stage.
    CanAdvance,

    /// The workflow is fully complete — nothing more to do.
    ///
    /// Returned when the same conditions as [`CanAdvance`] hold but the flow
    /// is already on the last stage of the workflow.
    ///
    /// [`CanAdvance`]: NextAction::CanAdvance
    Done,
}

// ---------------------------------------------------------------------------
// AdvanceBlocker
// ---------------------------------------------------------------------------

/// The reason the current stage cannot be advanced past yet.
///
/// Returned by [`can_advance`] as the `Err` payload when something prevents
/// the flow from moving to the next stage.
#[derive(Debug, PartialEq, Eq)]
pub enum AdvanceBlocker {
    /// The current stage's most-recent log entry is `fail`.
    ///
    /// The inner `String` is the stage name.
    StageFailed(String),

    /// The current stage has no `pass` log entry at all.
    ///
    /// The inner `String` is the stage name.
    StageIncomplete(String),

    /// A `produces` artifact declared by the current stage is missing from the
    /// artifact set.
    ///
    /// The inner `String` is the artifact path/name.
    MissingArtifact(String),
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Returns the highest-priority action to take next for the current stage.
///
/// The priority order is:
///
/// 1. **Failed command stage** — the current stage is a command kind and its
///    most-recent log entry is `fail`.
/// 2. **Incomplete stage** — the current stage has no `pass` entry: command
///    stages yield [`NextAction::RunCommand`], agent stages yield
///    [`NextAction::CompleteAgent`].
/// 3. **Missing artifact** — stage passed but a `produces` artifact is not yet
///    in `artifacts`. Returns [`NextAction::ProduceArtifact`].
/// 4. **Done / can advance** — stage passed and all artifacts are present.
///    Returns [`NextAction::Done`] on the last stage,
///    [`NextAction::CanAdvance`] otherwise.
///
/// # Panics
///
/// Panics if `flow.current_stage` does not name a stage in `workflow.stages`.
#[instrument(skip_all, fields(
    workflow.name = %workflow.name,
    flow.id = %flow.id,
    flow.current_stage = %flow.current_stage,
))]
pub fn next_action<'a>(
    workflow: &'a Workflow,
    flow: &FlowState,
    artifacts: &HashSet<String>,
) -> NextAction<'a> {
    let stage = current_stage(workflow, flow);
    let last_status = stage_last_status(flow, stage);

    // Priority 1: failed command stage.
    if matches!(stage.kind, StageKind::Command { .. })
        && last_status == Some(StageStatus::Failed)
    {
        return NextAction::RunCommand(stage);
    }

    // Priority 2: incomplete stage (no pass entry yet).
    if last_status != Some(StageStatus::Completed) {
        return match &stage.kind {
            StageKind::Command { .. } => NextAction::RunCommand(stage),
            StageKind::Agent { .. } => NextAction::CompleteAgent(stage),
        };
    }

    // Priority 3: all passed; check for missing artifacts.
    for artifact in &stage.produces {
        if !artifacts.contains(artifact) {
            return NextAction::ProduceArtifact(artifact.clone());
        }
    }

    // Priority 4: everything complete.
    if is_last_stage(workflow, flow) {
        NextAction::Done
    } else {
        NextAction::CanAdvance
    }
}

/// Checks whether the current stage is complete and the flow may advance.
///
/// Returns `Ok(())` when the current stage has a `pass` log entry and every
/// `produces` artifact is present in `artifacts`.
///
/// Returns `Err(blocker)` with the first [`AdvanceBlocker`] found, checked in
/// this order:
///
/// 1. [`AdvanceBlocker::StageFailed`] — the stage's most-recent entry is
///    `fail`.
/// 2. [`AdvanceBlocker::StageIncomplete`] — the stage has no `pass` entry.
/// 3. [`AdvanceBlocker::MissingArtifact`] — a `produces` artifact is absent.
///
/// # Panics
///
/// Panics if `flow.current_stage` does not name a stage in `workflow.stages`.
#[instrument(skip_all, fields(
    workflow.name = %workflow.name,
    flow.id = %flow.id,
    flow.current_stage = %flow.current_stage,
))]
pub fn can_advance(
    workflow: &Workflow,
    flow: &FlowState,
    artifacts: &HashSet<String>,
) -> Result<(), AdvanceBlocker> {
    let stage = current_stage(workflow, flow);

    match stage_last_status(flow, stage) {
        Some(StageStatus::Failed) => {
            return Err(AdvanceBlocker::StageFailed(stage.name.clone()));
        }
        Some(StageStatus::Completed) => {}
        None => {
            return Err(AdvanceBlocker::StageIncomplete(stage.name.clone()));
        }
        Some(_) => {
            unreachable!("stage_last_status only returns Completed or Failed")
        }
    }

    // Check that all declared artifacts have been produced.
    for artifact in &stage.produces {
        if !artifacts.contains(artifact) {
            return Err(AdvanceBlocker::MissingArtifact(artifact.clone()));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Finds the [`Stage`] in `workflow` whose name matches `flow.current_stage`.
///
/// # Panics
///
/// Panics if no stage with that name exists.
fn current_stage<'a>(workflow: &'a Workflow, flow: &FlowState) -> &'a Stage {
    workflow
        .stages
        .iter()
        .find(|s| s.name == flow.current_stage)
        .unwrap_or_else(|| {
            panic!(
                "current stage {:?} not found in workflow {:?}",
                flow.current_stage, workflow.name
            )
        })
}

/// Returns the status of the most-recent run entry for `stage` (i.e. the last
/// [`StageStatus::Completed`] or [`StageStatus::Failed`] entry), or `None` if
/// the stage has never been run.
fn stage_last_status(flow: &FlowState, stage: &Stage) -> Option<StageStatus> {
    flow.stage_history
        .iter()
        .rfind(|e| {
            e.stage == stage.name
                && matches!(
                    e.status,
                    StageStatus::Completed | StageStatus::Failed
                )
        })
        .map(|e| e.status)
}

/// Returns `true` if `flow` is currently on the last stage of `workflow`.
fn is_last_stage(workflow: &Workflow, flow: &FlowState) -> bool {
    workflow.stages.last().is_some_and(|s| s.name == flow.current_stage)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "actions.tests.rs"]
mod tests;
