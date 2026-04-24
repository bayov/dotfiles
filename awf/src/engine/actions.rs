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
use crate::model::workflow::LinearKind;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;

// ---------------------------------------------------------------------------
// NextAction
// ---------------------------------------------------------------------------

/// The highest-priority action to take next in the current flow.
#[derive(Debug, PartialEq, Eq)]
pub enum NextAction<'a> {
    /// Run (or re-run) this command stage.
    RunCommand(&'a Stage),

    /// Complete this agent stage.
    CompleteAgent(&'a Stage),

    /// Register the named artifact before advancing.
    ProduceArtifact(String),

    /// The stage is complete and all required artifacts are present — advance.
    CanAdvance,

    /// The workflow is fully complete — nothing more to do.
    Done,
}

// ---------------------------------------------------------------------------
// AdvanceBlocker
// ---------------------------------------------------------------------------

/// The reason the current stage cannot be advanced past yet.
#[derive(Debug, PartialEq, Eq)]
pub enum AdvanceBlocker {
    StageFailed(String),
    StageIncomplete(String),
    MissingArtifact(String),
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Returns the highest-priority action to take next for the current stage.
///
/// For task-runner stages, returns [`NextAction::CanAdvance`] or
/// [`NextAction::Done`] if all task lists are complete; otherwise callers
/// should delegate to the task-runner subcommands for detailed status.
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

    match &stage.kind {
        StageKind::TaskRunner { .. } => {
            // For task-runner stages, the completion criterion is that all task
            // lists are marked complete in task_runner_state. If any are not,
            // callers should use `awf flow tasks next` for details.
            if stage_is_complete_by_history(flow, stage) {
                if is_last_stage(workflow, flow) {
                    return NextAction::Done;
                }
                return NextAction::CanAdvance;
            }
            // No pass entry yet — signal that task-runner work is pending.
            // The CLI handles this specially via the tasks subcommand.
            return NextAction::CompleteAgent(stage);
        }
        StageKind::Linear(linear) => {
            // Priority 1: failed command stage.
            if matches!(linear, LinearKind::Command { .. })
                && last_status == Some(StageStatus::Failed)
            {
                return NextAction::RunCommand(stage);
            }

            // Priority 2: incomplete stage (no pass entry yet).
            if last_status != Some(StageStatus::Completed) {
                return match linear {
                    LinearKind::Command { .. } => NextAction::RunCommand(stage),
                    LinearKind::Agent { .. } => {
                        NextAction::CompleteAgent(stage)
                    }
                };
            }
        }
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

fn is_last_stage(workflow: &Workflow, flow: &FlowState) -> bool {
    workflow.stages.last().is_some_and(|s| s.name == flow.current_stage)
}

fn stage_is_complete_by_history(flow: &FlowState, stage: &Stage) -> bool {
    flow.stage_history
        .iter()
        .any(|e| e.stage == stage.name && e.status == StageStatus::Completed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "actions.tests.rs"]
mod tests;
