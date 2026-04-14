//! Handler for `awf flow new`.

use std::env;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use owo_colors::OwoColorize as _;

use super::AgentPrompt;
use super::FirstStageAction;
use super::FlowStatusStage;
use super::FlowStatusStageState;
use crate::cli::output::eprint_error;
use crate::cli::output::print_output;
use crate::model::workflow::StageKind;
use crate::result::Error;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Output payload for `awf flow new`.
#[derive(Debug)]
pub struct FlowNewOutput {
    pub path: String,
    pub workflow: String,
    pub stage: String,
    pub stages: Vec<FlowStatusStage>,
    pub first_stage_action: FirstStageAction,
}

impl fmt::Display for FlowNewOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Created `{}` flow: {}",
            self.workflow.blue(),
            self.path.cyan(),
        )?;

        writeln!(f)?;

        super::fmt_stage_list(
            f,
            &self.workflow,
            &self.stages,
            &std::collections::HashSet::new(),
        )?;

        writeln!(f)?;
        writeln!(f, "Next up (stage: {}):", self.stage.magenta())?;
        match &self.first_stage_action {
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
                    self.path.cyan()
                )?;
            }
            FirstStageAction::RunCommand => {
                writeln!(f, "    - Run: `awf flow run`")?;
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf flow new [--flow=<path>] --workflow=<workflow>`.
///
/// Resolves the flow path, loads the workflow to find its first stage, and
/// creates a new flow at the path.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if the workflow cannot be
/// loaded.
pub fn new(flow_arg: &str, workflow: &str) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let wf = crate::cli::load_workflow_or_exit(&entries, workflow)?;
    crate::cli::validate_workflow_or_exit(&wf, &entries, &config.commands);

    let flow_dir = resolve_new_flow_dir(flow_arg, &wf, &project_root, &config);

    let first_stage = wf.stages.first().map_or_else(
        || {
            eprint_error(format_args!(
                "workflow {workflow:?} has no stages; cannot create a flow"
            ));
            process::exit(1);
        },
        |s| s.name.as_str(),
    );

    let wf_dir = fs::find_workflow_dir(&entries, workflow).unwrap_or_default();

    match fs::create_flow(&flow_dir, workflow, first_stage) {
        Ok(()) => {
            let artifact_dir = flow_dir.join("artifacts");
            let stages: Vec<FlowStatusStage> = wf
                .stages
                .iter()
                .enumerate()
                .map(|(i, s)| FlowStatusStage {
                    name: s.name.clone(),
                    description: s.description.clone(),
                    produces: s
                        .produces
                        .iter()
                        .map(|name| artifact_dir.join(name))
                        .collect(),
                    state: if i == 0 {
                        FlowStatusStageState::Current
                    } else {
                        FlowStatusStageState::Pending { skippable: s.skippable }
                    },
                })
                .collect();
            let first_stage_action = match wf.stages.first().map(|s| &s.kind) {
                Some(StageKind::Agent { prompt, prompt_file }) => {
                    FirstStageAction::AgentComplete {
                        prompt: super::build_agent_prompt_from_fields(
                            prompt.as_deref(),
                            prompt_file.as_deref(),
                            &wf_dir,
                        ),
                    }
                }
                _ => FirstStageAction::RunCommand,
            };
            let output = FlowNewOutput {
                path: flow_dir.display().to_string(),
                workflow: workflow.to_string(),
                stage: first_stage.to_string(),
                stages,
                first_stage_action,
            };
            print_output(&output);
            Ok(())
        }
        Err(Error::FlowAlreadyExists { .. }) => {
            eprint_error(format_args!(
                "flow already exists at {}",
                flow_dir.display()
            ));
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Private helper
// ---------------------------------------------------------------------------

/// Resolve the flow directory from the positional argument to `flow new`.
///
/// - Absolute path (`/…`) or tilde (`~/…`): used directly.
/// - Relative path (`./…` or `../…`): resolved from the current directory.
/// - Bare name: joined to the workflow's `default_flow_dir` (which is itself
///   resolved relative to `project_root` if not absolute).
fn resolve_new_flow_dir(
    flow_arg: &str,
    workflow: &crate::model::workflow::Workflow,
    project_root: &Path,
    config: &crate::config::Config,
) -> PathBuf {
    use crate::store::resolve::TemplateError;
    use crate::store::resolve::expand_tilde;

    // Absolute or tilde path → use directly.
    if flow_arg.starts_with('/') || flow_arg.starts_with('~') {
        return expand_tilde(flow_arg);
    }

    // Explicit relative path → resolve from CWD.
    if flow_arg.starts_with("./") || flow_arg.starts_with("../") {
        return env::current_dir().expect("current_dir").join(flow_arg);
    }

    // Bare name → use workflow's default_flow_dir.
    let Some(ref default_dir) = workflow.default_flow_dir else {
        eprint_error(format_args!(
            "flow argument {flow_arg:?} looks like a name, not a path.\n\n  \
             To use a name the workflow must set `default_flow_dir` in its \
             WORKFLOW.toml.\n  Otherwise, provide a path: \
             ./{flow_arg}  or  /absolute/path/{flow_arg}"
        ));
        process::exit(1);
    };

    let expanded_dir = match crate::store::resolve::expand_template(
        default_dir,
        config.project_name.as_deref(),
    ) {
        Ok(s) => s,
        Err(TemplateError::ProjectNameNotConfigured) => {
            let config_path = project_root.join(".config/awf.toml");
            eprint_error(format_args!(
                "`{{project_name}}` requires `project_name` to be set in \
                 {}.\n\n  \
                 Add the following to that file:\n  \
                 project_name = \"<your-project-name>\"",
                config_path.display()
            ));
            process::exit(1);
        }
        Err(e) => {
            eprint_error(format_args!(
                "invalid default_flow_dir template: {e}"
            ));
            process::exit(1);
        }
    };
    let base = expand_tilde(&expanded_dir);
    let base = if base.is_relative() { project_root.join(base) } else { base };

    base.join(flow_arg)
}
