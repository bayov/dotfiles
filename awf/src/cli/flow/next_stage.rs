//! Handler for `awf flow next-stage`.

use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::Path;

use owo_colors::OwoColorize as _;

use super::AgentPrompt;
use super::FirstStageAction;
use super::FlowStatusStage;
use crate::cli::output::eprint_error;
use crate::cli::output::eprint_warning;
use crate::cli::output::print_output;
use crate::engine::actions::AdvanceBlocker;
use crate::engine::actions::can_advance;
use crate::engine::state::advance_stage;
use crate::engine::state::record_stage_pass;
use crate::engine::state::skip_stage;
use crate::model::flow::FlowState;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Output payload for `awf flow done` (final agent stage).
#[derive(Debug)]
pub struct FlowDoneOutput {
    pub stage: String,
    pub flow_path: String,
}

impl fmt::Display for FlowDoneOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} Stage {} marked complete.",
            "✓".green(),
            self.stage.magenta()
        )?;
        writeln!(f)?;
        writeln!(
            f,
            "Workflow complete. Use `awf flow archive --flow={}` to archive it.",
            self.flow_path.cyan()
        )
    }
}

/// Output payload for `awf flow next-stage`.
#[derive(Debug)]
pub struct FlowAdvanceOutput {
    pub stage: String,
    pub is_final: bool,
    pub flow_path: String,
    pub workflow: String,
    pub stages: Vec<FlowStatusStage>,
    pub next_action: FirstStageAction,
}

impl fmt::Display for FlowAdvanceOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {}", "▶ Entered stage:".green(), self.stage.magenta())?;
        if self.is_final {
            writeln!(f, "{}", "  This is the final stage.".bright_black())?;
        }
        writeln!(f)?;
        super::fmt_stage_list(f, &self.workflow, &self.stages)?;
        writeln!(f)?;
        writeln!(f, "Next up (stage: {}):", self.stage.magenta())?;
        match &self.next_action {
            FirstStageAction::AgentComplete { prompt } => {
                match prompt {
                    Some(AgentPrompt::Inline(text)) => {
                        writeln!(f, "    - Instructions: {text}")?;
                    }
                    Some(AgentPrompt::File(path)) => {
                        writeln!(
                            f,
                            "    - Follow the instructions at: {}",
                            path.display()
                        )?;
                    }
                    None => {}
                }
                writeln!(
                    f,
                    "    - When done, proceed to the next stage with: `awf flow next-stage --flow={}`",
                    self.flow_path.cyan()
                )?;
            }
            FirstStageAction::RunCommand => {
                writeln!(
                    f,
                    "    - Run: `awf flow run --flow={}`",
                    self.flow_path.cyan()
                )?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf flow next-stage [--force] [--skip] [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn advance(
    flow_flag: Option<&str>,
    force: Option<String>,
    skip: bool,
) -> color_eyre::Result<()> {
    if skip && force.is_some() {
        eprint_error("--skip and --force cannot be used together");
        std::process::exit(1);
    }

    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = super::resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = super::load_flow_or_exit(&flow_dir)?;
    let workflow = crate::cli::load_workflow_or_exit(&entries, &flow.workflow)?;
    crate::cli::validate_workflow_or_exit(
        &workflow,
        &entries,
        &config.commands,
    );

    // For agent stages (and not --skip): auto-record completion before
    // advancing. This replaces the former `complete-stage` command.
    // The completion is saved to disk immediately so that if the advance
    // fails (e.g. missing artifact) the stage is still recorded as done.
    let flow = if skip {
        flow
    } else {
        maybe_complete_agent(&workflow, flow, &flow_dir)?
    };

    // Final agent stage: completion was recorded above; nothing left to
    // advance to, so report success and return.
    if !skip && force.is_none() && is_final_agent_stage(&workflow, &flow) {
        let output = FlowDoneOutput {
            stage: flow.current_stage,
            flow_path: flow_dir.display().to_string(),
        };
        print_output(&output);
        return Ok(());
    }

    let updated_flow = if skip {
        // --skip bypasses all gates (stage completion and artifacts).
        let skipped_stage = flow.current_stage.clone();
        match skip_stage(&workflow, flow) {
            Ok(f) => {
                eprint_warning(format_args!("Skipped stage {skipped_stage:?}"));
                f
            }
            Err(Error::StageNotSkippable { stage }) => {
                eprint_error(format_args!(
                    "stage `{stage}` is not marked as skippable in the workflow"
                ));
                std::process::exit(1);
            }
            Err(Error::AlreadyOnLastStage) => {
                eprint_error(
                    "already on the last stage; nothing to advance to",
                );
                std::process::exit(1);
            }
            Err(e) => return Err(e.into()),
        }
    } else {
        let artifacts = super::collect_artifacts(&flow_dir);

        // --force bypasses stage gates but never artifact gates.
        #[allow(clippy::single_match_else, clippy::option_if_let_else)]
        let force_reason = match force {
            Some(reason) => {
                check_produces_or_exit(&workflow, &flow, &artifacts);
                Some(reason)
            }
            None => {
                check_can_advance_or_exit(&workflow, &flow, &artifacts);
                None
            }
        };

        match advance_stage(&workflow, flow, force_reason) {
            Ok(f) => f,
            Err(Error::AlreadyOnLastStage) => {
                eprint_error(
                    "already on the last stage; nothing to advance to",
                );
                std::process::exit(1);
            }
            Err(e) => return Err(e.into()),
        }
    };

    fs::save_flow(&flow_dir, &updated_flow)?;

    let wf_dir =
        fs::find_workflow_dir(&entries, &workflow.name).unwrap_or_default();
    let output =
        build_advance_output(&workflow, &updated_flow, &flow_dir, &wf_dir);
    print_output(&output);
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn check_can_advance_or_exit(
    workflow: &Workflow,
    flow: &FlowState,
    artifacts: &HashSet<String>,
) {
    if let Err(blocker) = can_advance(workflow, flow, artifacts) {
        match blocker {
            AdvanceBlocker::StageFailed(name) => {
                eprint_error(format_args!(
                    "stage `{name}` failed. Fix the error then \
                     run `awf flow run` before advancing."
                ));
            }
            AdvanceBlocker::StageIncomplete(name) => {
                eprint_error(format_args!(
                    "stage `{name}` is not complete. Complete it \
                     before advancing."
                ));
            }
            AdvanceBlocker::MissingArtifact(name) => {
                eprint_error(format_args!(
                    "Missing artifact `{name}`. Register it with \
                     `awf flow artifact put {name} <file>` before \
                     advancing."
                ));
            }
        }
        std::process::exit(1);
    }
}

/// Checks that all `produces` artifacts for the current stage exist, exiting
/// with an error if any are missing. Used by `--force` which bypasses stage
/// gates but still requires artifacts.
fn check_produces_or_exit(
    workflow: &Workflow,
    flow: &FlowState,
    artifacts: &HashSet<String>,
) {
    let Some(stage) =
        workflow.stages.iter().find(|s| s.name == flow.current_stage)
    else {
        return;
    };
    for artifact in &stage.produces {
        if !artifacts.contains(artifact) {
            eprint_error(format_args!(
                "Missing artifact `{artifact}`. Register it with \
                 `awf flow artifact put {artifact} <file>` before advancing."
            ));
            std::process::exit(1);
        }
    }
}

/// Builds the `FlowAdvanceOutput` for the newly entered stage.
fn build_advance_output(
    workflow: &Workflow,
    updated_flow: &FlowState,
    flow_dir: &Path,
    wf_dir: &Path,
) -> FlowAdvanceOutput {
    let new_stage = &updated_flow.current_stage;
    let is_final = workflow.stages.last().is_some_and(|s| s.name == *new_stage);
    let stages = super::build_stage_list(workflow, updated_flow);
    let next_action = workflow
        .stages
        .iter()
        .find(|s| s.name == *new_stage)
        .map_or(FirstStageAction::RunCommand, |s| match &s.kind {
            StageKind::Agent { prompt, prompt_file } => {
                FirstStageAction::AgentComplete {
                    prompt: super::build_agent_prompt_from_fields(
                        prompt.as_deref(),
                        prompt_file.as_deref(),
                        wf_dir,
                    ),
                }
            }
            StageKind::Command { .. } => FirstStageAction::RunCommand,
        });
    FlowAdvanceOutput {
        stage: new_stage.clone(),
        is_final,
        flow_path: flow_dir.display().to_string(),
        workflow: workflow.name.clone(),
        stages,
        next_action,
    }
}

/// Auto-completes an agent stage that has not yet been recorded as complete.
///
/// Saves the updated flow to disk immediately so that if the subsequent
/// advance fails (e.g. missing artifact) the completion is preserved.
/// Returns the (possibly updated) flow unchanged for non-agent stages or
/// stages that are already complete.
fn maybe_complete_agent(
    workflow: &Workflow,
    flow: FlowState,
    flow_dir: &Path,
) -> color_eyre::Result<FlowState> {
    let stage = super::current_stage_or_exit(workflow, &flow);
    if matches!(stage.kind, StageKind::Agent { .. })
        && !stage_is_complete(&flow, &stage.name)
    {
        let completed = record_stage_pass(flow, stage, None);
        fs::save_flow(flow_dir, &completed)?;
        Ok(completed)
    } else {
        Ok(flow)
    }
}

/// Returns `true` when the current stage is an agent stage that is the last
/// stage in the workflow.
fn is_final_agent_stage(workflow: &Workflow, flow: &FlowState) -> bool {
    let stage = super::current_stage_or_exit(workflow, flow);
    matches!(stage.kind, StageKind::Agent { .. })
        && workflow.stages.last().is_some_and(|s| s.name == flow.current_stage)
}

/// Returns `true` if `stage_name` has a `Completed` entry in the flow history.
fn stage_is_complete(flow: &FlowState, stage_name: &str) -> bool {
    flow.stage_history.iter().rev().any(|e| {
        e.stage == stage_name
            && e.status == crate::model::flow::StageStatus::Completed
    })
}
