//! Handler for `awf flow status`.

use std::env;
use std::fmt;

use owo_colors::OwoColorize as _;

use super::AgentPrompt;
use super::FirstStageAction;
use super::FlowStatusStage;
use crate::cli::output::print_output;
use crate::engine::actions::NextAction;
use crate::engine::actions::next_action;
use crate::model::workflow::StageKind;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Flow-level status for `awf flow status`.
#[derive(Debug, PartialEq, Eq)]
pub enum FlowStatusLevel {
    /// Flow is in progress.
    Active,
    /// All stages have been completed (or skipped).
    Completed,
    /// Flow has been archived.
    Archived,
}

/// The complete status payload for `awf flow status`.
#[derive(Debug)]
pub struct FlowStatusOutput {
    pub flow_path: String,
    pub workflow: String,
    pub stages: Vec<FlowStatusStage>,
    pub current_stage: String,
    pub status: FlowStatusLevel,
    pub next_action: Option<FirstStageAction>,
}

impl fmt::Display for FlowStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Header
        writeln!(f, "    Flow: {}", self.flow_path.cyan())?;
        writeln!(f, "Workflow: {}", self.workflow.blue())?;
        let status_str = match self.status {
            FlowStatusLevel::Active => {
                format!(
                    "{} (stage: {})",
                    "active".bold().blue(),
                    self.current_stage.magenta()
                )
            }
            FlowStatusLevel::Completed => {
                "completed".bold().green().to_string()
            }
            FlowStatusLevel::Archived => {
                format!(
                    "{} (stage: {})",
                    "archived".bold().yellow(),
                    self.current_stage.magenta()
                )
            }
        };
        writeln!(f, "  Status: {status_str}")?;
        writeln!(f)?;

        super::fmt_stage_list(f, &self.workflow, &self.stages)?;

        match self.status {
            FlowStatusLevel::Active => {
                if let Some(action) = &self.next_action {
                    writeln!(f)?;
                    writeln!(
                        f,
                        "Next up (stage: {}):",
                        self.current_stage.magenta()
                    )?;
                    match action {
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
                }
            }
            FlowStatusLevel::Completed => {
                writeln!(f)?;
                writeln!(
                    f,
                    "Workflow complete. To archive this flow, run: `awf flow archive --flow={}`",
                    self.flow_path.cyan()
                )?;
            }
            FlowStatusLevel::Archived => {
                writeln!(f)?;
                writeln!(
                    f,
                    "This flow is {} and cannot be modified. To re-activate the flow, run: `awf flow unarchive --flow={}`",
                    "archived".bold().yellow(),
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

/// Handles `awf flow status [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if the workflow/flow cannot
/// be loaded.
pub fn status(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = super::resolve_or_exit(flow_flag, &config)?;
    let flow = super::load_flow_or_exit(&flow_dir)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);
    let workflow = crate::cli::load_workflow_or_exit(&entries, &flow.workflow)?;
    crate::cli::validate_workflow_or_exit(
        &workflow,
        &entries,
        &config.commands,
    );
    let wf_dir =
        fs::find_workflow_dir(&entries, &flow.workflow).unwrap_or_default();

    let current_idx = workflow
        .stages
        .iter()
        .position(|s| s.name == flow.current_stage)
        .unwrap_or(0);

    let artifacts = super::collect_artifacts(&flow_dir);
    let flow_action = next_action(&workflow, &flow, &artifacts);

    let status = if flow.archived {
        FlowStatusLevel::Archived
    } else if matches!(flow_action, NextAction::Done) {
        FlowStatusLevel::Completed
    } else {
        FlowStatusLevel::Active
    };

    let stages = super::build_stage_list(&workflow, &flow);

    let next_action_for_display = match status {
        FlowStatusLevel::Active => {
            workflow.stages.get(current_idx).map(|s| match &s.kind {
                StageKind::Agent { prompt, prompt_file } => {
                    FirstStageAction::AgentComplete {
                        prompt: super::build_agent_prompt_from_fields(
                            prompt.as_deref(),
                            prompt_file.as_deref(),
                            &wf_dir,
                        ),
                    }
                }
                StageKind::Command { .. } => FirstStageAction::RunCommand,
            })
        }
        _ => None,
    };

    let output = FlowStatusOutput {
        flow_path: flow_dir.display().to_string(),
        workflow: flow.workflow.clone(),
        stages,
        current_stage: flow.current_stage.clone(),
        status,
        next_action: next_action_for_display,
    };

    print_output(&output);
    Ok(())
}
