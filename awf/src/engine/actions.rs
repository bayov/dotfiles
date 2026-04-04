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
use crate::model::flow::StepStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::Step;
use crate::model::workflow::StepKind;
use crate::model::workflow::Workflow;

// ---------------------------------------------------------------------------
// NextAction
// ---------------------------------------------------------------------------

/// The highest-priority action to take next in the current flow.
///
/// Variants are listed in priority order — [`RunStep`] for a failing command
/// step has the highest urgency, while [`Done`] signals that the entire
/// workflow is complete.
///
/// [`RunStep`]: NextAction::RunStep
/// [`Done`]: NextAction::Done
#[derive(Debug, PartialEq, Eq)]
pub enum NextAction<'a> {
    /// Re-run (or run) this command step.
    ///
    /// Returned when a command step in the current stage has a `fail` status
    /// as its most recent log entry (highest priority) or has not yet been run
    /// at all.
    RunStep(&'a Step),

    /// Complete this agent step.
    ///
    /// Returned when an agent step in the current stage has not yet received a
    /// `pass` log entry (i.e. it has not been completed by the agent).
    CompleteAgentStep(&'a Step),

    /// Register the named artifact before advancing.
    ///
    /// Returned when all steps in the current stage have passed but one of the
    /// stage's declared `produces` artifacts is absent from the artifact set.
    ProduceArtifact(String),

    /// All steps passed and all required artifacts are present — advance now.
    ///
    /// Returned when every step in the current stage has a `pass` log entry,
    /// every `produces` artifact is in the artifact set, and the flow is
    /// **not** on the last stage.
    CanAdvance,

    /// The workflow is fully complete — nothing more to do.
    ///
    /// Returned when the same conditions as [`CanAdvance`] hold but the flow is
    /// already on the last stage of the workflow.
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
    /// A step's most-recent log entry for the current stage is `fail`.
    ///
    /// The inner `String` is the step name.
    StepFailed(String),

    /// A step has no `pass` log entry for the current stage at all.
    ///
    /// The inner `String` is the step name.
    StepIncomplete(String),

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
/// 1. **Failed command step** — a command step whose most-recent log entry for
///    this stage is `fail`. Address failures before moving on.
/// 2. **Incomplete step** — any step (command or agent) with no `pass` entry
///    for this stage: command steps yield [`NextAction::RunStep`], agent steps
///    yield [`NextAction::CompleteAgentStep`].
/// 3. **Missing artifact** — all steps passed but a `produces` artifact from
///    the stage is not yet in `artifacts`. Returns
///    [`NextAction::ProduceArtifact`].
/// 4. **Done / can advance** — all steps passed and all artifacts are present.
///    Returns [`NextAction::Done`] on the last stage,
///    [`NextAction::CanAdvance`] otherwise.
///
/// # Panics
///
/// Panics if `flow.current_stage` does not name a stage in `workflow.stages`.
/// Callers should ensure the flow is consistent with the workflow before
/// calling this function.
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

    // Priority 1 & 2: scan steps in definition order.
    //
    // We do two passes so that a failing command step (priority 1) is always
    // surfaced before any merely-incomplete step (priority 2), regardless of
    // definition order.

    // Pass A — find the first failing command step.
    for step in &stage.steps {
        if is_command_step(step)
            && step_last_status(flow, stage, step) == Some(StepStatus::Fail)
        {
            return NextAction::RunStep(step);
        }
    }

    // Pass B — find the first step that has not yet passed.
    for step in &stage.steps {
        if step_last_status(flow, stage, step) != Some(StepStatus::Pass) {
            return match &step.kind {
                StepKind::Command { .. } => NextAction::RunStep(step),
                StepKind::Agent { .. } => NextAction::CompleteAgentStep(step),
            };
        }
    }

    // Priority 3 — all steps passed; check for missing artifacts.
    for artifact in &stage.produces {
        if !artifacts.contains(artifact) {
            return NextAction::ProduceArtifact(artifact.clone());
        }
    }

    // Priority 4 — everything complete.
    if is_last_stage(workflow, flow) {
        NextAction::Done
    } else {
        NextAction::CanAdvance
    }
}

/// Checks whether the current stage is complete and the flow may advance.
///
/// Returns `Ok(())` when every step in the current stage has a `pass` log
/// entry and every `produces` artifact is present in `artifacts`.
///
/// Returns `Err(blocker)` with the first [`AdvanceBlocker`] found, checked in
/// this order:
///
/// 1. [`AdvanceBlocker::StepFailed`] — a step's most-recent entry is `fail`.
/// 2. [`AdvanceBlocker::StepIncomplete`] — a step has no `pass` entry at all.
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

    // Check each step: fail takes priority over incomplete.
    for step in &stage.steps {
        match step_last_status(flow, stage, step) {
            Some(StepStatus::Fail) => {
                return Err(AdvanceBlocker::StepFailed(step.name.clone()));
            }
            Some(StepStatus::Pass) => {}
            None => {
                return Err(AdvanceBlocker::StepIncomplete(step.name.clone()));
            }
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

/// Returns the status of the most-recent log entry for `step` in `stage`, or
/// `None` if the step has never been executed during the current stage visit.
///
/// Only entries whose `stage` field matches `stage.name` are considered, so
/// log entries from a previous visit to the same stage (when `repeatable`) are
/// not included — the caller is responsible for clearing those if needed.
/// However, for v1 all log entries for the current stage name are used.
fn step_last_status(
    flow: &FlowState,
    stage: &Stage,
    step: &Step,
) -> Option<StepStatus> {
    flow.step_log
        .iter()
        .rfind(|e| e.stage == stage.name && e.step == step.name)
        .map(|e| e.status)
}

/// Returns `true` if `step` is a command step.
const fn is_command_step(step: &Step) -> bool {
    matches!(step.kind, StepKind::Command { .. })
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
