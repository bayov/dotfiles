//! Handler for `awf flow next-step`.

use std::env;
use std::fmt;
use std::path::PathBuf;

use owo_colors::OwoColorize as _;

use crate::cli::output::print_output;
use crate::engine::actions::NextAction;
use crate::engine::actions::next_action;
use crate::model::workflow::StageKind;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Output payload for `awf flow next-step`.
///
/// Each entry in `lines` is printed as a separate `→ {line}` row.
#[derive(Debug)]
pub struct FlowNextOutput {
    pub lines: Vec<String>,
}

impl fmt::Display for FlowNextOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{} {}", "→".cyan(), line)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf flow next-step [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn next(flow_flag: Option<&str>) -> color_eyre::Result<()> {
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
    let artifacts = super::collect_artifacts(&flow_dir);

    let output = match next_action(&workflow, &flow, &artifacts) {
        NextAction::RunCommand(stage) => FlowNextOutput {
            lines: vec![format!(
                "[command] {} \u{2014} run: awf flow run",
                stage.name
            )],
        },
        NextAction::CompleteAgent(stage) => {
            let mut lines = Vec::new();
            match &stage.kind {
                StageKind::Agent { prompt: Some(text), .. } => {
                    lines.push(format!("Instructions: {text}"));
                }
                StageKind::Agent { prompt_file: Some(rel_or_abs), .. } => {
                    let wf_dir =
                        fs::find_workflow_dir(&entries, &flow.workflow)
                            .unwrap_or_default();
                    let path = if rel_or_abs.starts_with('/') {
                        PathBuf::from(rel_or_abs)
                    } else {
                        wf_dir.join(rel_or_abs)
                    };
                    lines.push(format!(
                        "Follow the instructions at: {}",
                        path.display()
                    ));
                }
                _ => {}
            }
            lines.push("Mark complete with: awf flow next-stage".to_string());
            FlowNextOutput { lines }
        }
        NextAction::ProduceArtifact(name) => FlowNextOutput {
            lines: vec![
                format!("Stage complete. Produce artifact: {name}"),
                "Write it to the artifacts/ directory inside the flow path."
                    .to_string(),
            ],
        },
        NextAction::CanAdvance => {
            let next_stage_name = workflow
                .stages
                .iter()
                .position(|s| s.name == flow.current_stage)
                .and_then(|idx| workflow.stages.get(idx + 1))
                .map_or_else(|| "next".to_string(), |s| s.name.clone());
            FlowNextOutput {
                lines: vec![format!(
                    "Stage complete. Run `awf flow next-stage` to move to \
                     {next_stage_name}."
                )],
            }
        }
        NextAction::Done => FlowNextOutput {
            lines: vec![
                "All stages complete. The workflow is done.".to_string(),
            ],
        },
    };

    print_output(&output);
    Ok(())
}
