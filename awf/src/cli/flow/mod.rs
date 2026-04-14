//! Handlers for `awf flow` subcommands.

pub mod archive;
pub mod history;
pub mod list;
pub mod new;
pub mod next_stage;
pub mod run;
pub mod status;
pub mod validate;

use std::collections::HashSet;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::process;

pub use archive::archive;
pub use history::history;
pub use list::list;
pub use new::new;
pub use next_stage::advance;
use owo_colors::OwoColorize as _;
pub use run::run;
pub use status::status;
pub use validate::validate;

use crate::cli::output::eprint_error;
use crate::model::flow::FlowState;
use crate::model::workflow::Stage;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::store::fs;
use crate::store::resolve::resolve_flow_path;

// ---------------------------------------------------------------------------
// Shared output types
// ---------------------------------------------------------------------------

/// The prompt payload for an agent stage.
#[derive(Debug)]
pub enum AgentPrompt {
    /// Inline instruction string passed directly to the agent.
    Inline(String),
    /// Resolved absolute path to a prompt file.
    File(PathBuf),
}

/// What the agent should do next for the first stage.
#[derive(Debug)]
pub enum FirstStageAction {
    AgentComplete { prompt: Option<AgentPrompt> },
    RunCommand,
}

/// Per-stage data for the stage-list display, shared by `awf flow new` and
/// `awf flow status`.
#[derive(Debug)]
pub struct FlowStatusStage {
    pub name: String,
    pub description: Option<String>,
    /// Resolved absolute paths to each artifact this stage produces.
    pub produces: Vec<PathBuf>,
    pub state: FlowStatusStageState,
}

/// Annotation for a stage's state in the status display.
#[derive(Debug)]
pub enum FlowStatusStageState {
    Completed,
    ForceCompleted,
    Skipped,
    Current,
    Pending { skippable: bool },
}

/// Builds the [`FlowStatusStage`] list for a workflow + flow state, shared by
/// `awf flow status` and `awf flow next-stage`.
///
/// `flow_dir` is used to resolve artifact names to absolute paths
/// (`<flow_dir>/artifacts/<name>`).
pub fn build_stage_list(
    workflow: &Workflow,
    flow: &FlowState,
    flow_dir: &Path,
) -> Vec<FlowStatusStage> {
    let artifact_dir = flow_dir.join("artifacts");
    let current_idx = workflow
        .stages
        .iter()
        .position(|s| s.name == flow.current_stage)
        .unwrap_or(0);
    workflow
        .stages
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let state = if i <= current_idx {
                let last =
                    flow.stage_history.iter().rfind(|e| e.stage == s.name);
                match last.map(|e| e.status) {
                    Some(crate::model::flow::StageStatus::Skipped) => {
                        FlowStatusStageState::Skipped
                    }
                    Some(crate::model::flow::StageStatus::ForceCompleted) => {
                        FlowStatusStageState::ForceCompleted
                    }
                    Some(crate::model::flow::StageStatus::Completed) => {
                        FlowStatusStageState::Completed
                    }
                    _ => FlowStatusStageState::Current,
                }
            } else {
                FlowStatusStageState::Pending { skippable: s.skippable }
            };
            FlowStatusStage {
                name: s.name.clone(),
                description: s.description.clone(),
                produces: s
                    .produces
                    .iter()
                    .map(|name| artifact_dir.join(name))
                    .collect(),
                state,
            }
        })
        .collect()
}

/// Renders the "Workflow `foo` has N stages: ..." stage-list block, shared by
/// `awf flow new`, `awf flow status`, and `awf flow next-stage`.
///
/// `artifacts` is the set of artifact filenames present in the flow's
/// artifacts directory; used to distinguish skipped-but-present artifacts.
pub(super) fn fmt_stage_list(
    f: &mut fmt::Formatter<'_>,
    workflow: &str,
    stages: &[FlowStatusStage],
    artifacts: &HashSet<String>,
) -> fmt::Result {
    writeln!(f, "Workflow `{}` has {} stages:", workflow.blue(), stages.len())?;
    for (i, stage) in stages.iter().enumerate() {
        let (prefix, suffix) = match &stage.state {
            FlowStatusStageState::Completed => {
                ("✓ ".green().to_string(), " (completed)".green().to_string())
            }
            FlowStatusStageState::ForceCompleted => (
                "✓ ".yellow().to_string(),
                " (force-completed)".yellow().to_string(),
            ),
            FlowStatusStageState::Skipped => (
                "○ ".bright_black().to_string(),
                " (skipped)".bright_black().to_string(),
            ),
            FlowStatusStageState::Current => {
                ("▶ ".blue().to_string(), " (current)".blue().to_string())
            }
            FlowStatusStageState::Pending { skippable } => (
                "  ".to_string(),
                if *skippable {
                    " (skippable)".to_string()
                } else {
                    String::new()
                },
            ),
        };
        writeln!(f, "{prefix}{}. {}{suffix}", i + 1, stage.name.magenta())?;

        if let Some(desc) = &stage.description {
            writeln!(f, "    - {desc}")?;
        }
        for artifact in &stage.produces {
            let name =
                artifact.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let label = match &stage.state {
                FlowStatusStageState::Completed
                | FlowStatusStageState::ForceCompleted => "Produced:",
                FlowStatusStageState::Skipped => {
                    if artifacts.contains(name) {
                        "Manually produced:"
                    } else {
                        "Skipped artifact:"
                    }
                }
                FlowStatusStageState::Current
                | FlowStatusStageState::Pending { .. } => "Produces:",
            };
            writeln!(f, "    - {label} {}", artifact.display())?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Resolves the flow path, printing a friendly error and exiting if none is
/// available.
fn resolve_or_exit(
    flow_flag: Option<&str>,
    config: &crate::config::Config,
) -> color_eyre::Result<PathBuf> {
    match resolve_flow_path(flow_flag, config) {
        Ok(path) => Ok(path),
        Err(Error::FlowPathRequired) => {
            eprint_error(
                "no flow specified; pass --flow=<path> or set AWF_ACTIVE_FLOW",
            );
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

/// Loads a flow state, printing a friendly error and exiting if not found.
fn load_flow_or_exit(flow_dir: &Path) -> color_eyre::Result<FlowState> {
    match fs::load_flow(flow_dir) {
        Ok(flow) => Ok(flow),
        Err(Error::FlowNotFound { .. }) => {
            eprint_error(format_args!(
                "flow not found at {}",
                flow_dir.display()
            ));
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

/// Scans `<flow_dir>/artifacts/` and returns the set of filenames present.
fn collect_artifacts(flow_dir: &Path) -> HashSet<String> {
    let artifact_dir = flow_dir.join("artifacts");
    let Ok(read_dir) = std::fs::read_dir(&artifact_dir) else {
        return HashSet::new();
    };
    let mut names = HashSet::new();
    for entry in read_dir.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            names.insert(name.to_string());
        }
    }
    names
}

/// Builds an [`AgentPrompt`] from raw `prompt`/`prompt_file` field values and
/// the workflow directory.
fn build_agent_prompt_from_fields(
    prompt: Option<&str>,
    prompt_file: Option<&str>,
    wf_dir: &Path,
) -> Option<AgentPrompt> {
    if let Some(text) = prompt {
        return Some(AgentPrompt::Inline(text.to_owned()));
    }
    if let Some(rel_or_abs) = prompt_file {
        let path = if rel_or_abs.starts_with('/') {
            PathBuf::from(rel_or_abs)
        } else {
            wf_dir.join(rel_or_abs)
        };
        return Some(AgentPrompt::File(path));
    }
    None
}

/// Finds the current stage in the workflow, exiting if not found.
fn current_stage_or_exit<'a>(
    workflow: &'a Workflow,
    flow: &FlowState,
) -> &'a Stage {
    workflow
        .stages
        .iter()
        .find(|s| s.name == flow.current_stage)
        .unwrap_or_else(|| {
            eprint_error(format_args!(
                "current stage {:?} not found in workflow {:?}",
                flow.current_stage, workflow.name
            ));
            process::exit(1);
        })
}
