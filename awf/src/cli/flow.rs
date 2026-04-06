//! Handlers for `awf flow` subcommands.

use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use chrono::Local;
use chrono::Utc;
use owo_colors::OwoColorize as _;
use serde::Serialize;

use crate::cli::output::OutputFormat;
use crate::cli::output::eprint_error;
use crate::cli::output::eprint_warning;
use crate::cli::output::print_output;
use crate::engine::actions::AdvanceBlocker;
use crate::engine::actions::NextAction;
use crate::engine::actions::can_advance;
use crate::engine::actions::next_action;
use crate::engine::command::run_command;
use crate::engine::state::EnterResult;
use crate::engine::state::advance_stage;
use crate::engine::state::can_enter_stage;
use crate::engine::state::record_step_fail;
use crate::engine::state::record_step_pass;
use crate::model::flow::FlowState;
use crate::model::flow::StepStatus;
use crate::model::workflow::Step;
use crate::model::workflow::StepKind;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::store::fs;
use crate::store::resolve::resolve_flow_path;

// ---------------------------------------------------------------------------
// Output types — `awf flow new`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow new`.
#[derive(Debug, Serialize)]
pub struct FlowNewOutput {
    pub path: String,
    pub workflow: String,
    pub stage: String,
}

impl fmt::Display for FlowNewOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Created flow: {} (workflow: {}, stage: {})",
            self.path.cyan(),
            self.workflow.blue(),
            self.stage.magenta(),
        )
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow list`
// ---------------------------------------------------------------------------

/// A single flow entry in the list output.
#[derive(Debug, Serialize)]
pub struct FlowListEntry {
    pub path: String,
    pub workflow: String,
    pub stage: String,
    pub archived: bool,
}

/// Output payload for `awf flow list`.
#[derive(Debug, Serialize)]
pub struct FlowListOutput {
    pub flows: Vec<FlowListEntry>,
    /// Controls the empty-list hint and whether the ARCHIVED column is shown.
    /// Skipped from serialisation — this is a query parameter, not output
    /// data.
    #[serde(skip)]
    pub include_archived: bool,
}

impl fmt::Display for FlowListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.flows.is_empty() {
            if self.include_archived {
                return writeln!(f, "No flows found.");
            }
            return writeln!(
                f,
                "No flows found. Create one with: awf flow new \
                 --flow <path> --workflow <workflow>"
            );
        }

        for entry in &self.flows {
            write!(
                f,
                "• {}  (workflow: {}, stage: {}",
                entry.path.cyan(),
                entry.workflow.blue(),
                entry.stage.magenta(),
            )?;
            if self.include_archived && entry.archived {
                write!(f, ", {}", "archived".bright_black())?;
            }
            writeln!(f, ")")?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow status`
// ---------------------------------------------------------------------------

/// A single step's status as computed for the `status` command output.
#[derive(Debug, Serialize)]
pub struct StepStatusLine {
    pub name: String,
    pub status: String,
}

/// The complete status payload for `awf flow status`.
#[derive(Debug, Serialize)]
pub struct FlowStatusOutput {
    pub flow: String,
    pub stage: String,
    pub steps: Vec<StepStatusLine>,
}

impl fmt::Display for FlowStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Flow:  {}", self.flow.cyan())?;
        writeln!(f, "Stage:  {}", self.stage.magenta())?;
        writeln!(f, "Steps:")?;
        for step in &self.steps {
            let badge = match step.status.as_str() {
                "pass" => "[✓ pass]".green().to_string(),
                "fail" => "[✗ fail]".red().to_string(),
                _ => "[· pending]".bright_black().to_string(),
            };
            writeln!(f, "  {}  {badge}", step.name.magenta())?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow next-step`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow next-step`.
#[derive(Debug, Serialize)]
pub struct FlowNextOutput {
    /// Discriminator: `"run_step"`, `"complete_agent_step"`,
    /// `"produce_artifact"`, `"can_advance"`, or `"done"`.
    pub action: String,
    /// The step name (for `run_step` and `complete_agent_step`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    /// The artifact name (for `produce_artifact`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    /// The name of the next stage (for `can_advance`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_stage: Option<String>,
    /// The raw prompt file contents (for `complete_agent_step`, when
    /// available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Human-readable instruction.
    pub message: String,
}

impl fmt::Display for FlowNextOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref prompt) = self.prompt {
            // Replicate `println!("{prompt}")`: write contents then a newline.
            writeln!(f, "{prompt}")?;
        }
        writeln!(f, "{} {}", "→".cyan(), self.message)
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow run`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow run`.
#[derive(Debug, Serialize)]
pub struct FlowRunOutput {
    pub step: String,
    pub command: String,
    /// `"pass"` or `"fail"`.
    pub status: String,
    pub duration_ms: u64,
}

impl fmt::Display for FlowRunOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_badge = if self.status == "pass" {
            "PASS".green().to_string()
        } else {
            "FAIL".red().to_string()
        };
        writeln!(
            f,
            "{status_badge}  {}  ({}ms)",
            self.command, self.duration_ms
        )?;
        if self.status == "fail" {
            writeln!(
                f,
                "Fix the error, then run: awf flow run {}",
                self.step.magenta()
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow step done`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow step done`.
#[derive(Debug, Serialize)]
pub struct FlowStepDoneOutput {
    pub step: String,
}

impl fmt::Display for FlowStepDoneOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} Step {} marked complete.",
            "✓".green(),
            self.step.magenta()
        )
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow validate`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow validate` (success path).
#[derive(Debug, Serialize)]
pub struct FlowValidateOutput {
    pub valid: bool,
}

impl fmt::Display for FlowValidateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "✓ Flow artifacts valid.".green())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow next-stage`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow next-stage`.
#[derive(Debug, Serialize)]
pub struct FlowAdvanceOutput {
    pub stage: String,
    pub is_final: bool,
}

impl fmt::Display for FlowAdvanceOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {}", "▶ Entered stage:".green(), self.stage.magenta())?;
        if self.is_final {
            writeln!(f, "{}", "  This is the final stage.".bright_black())?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow history`
// ---------------------------------------------------------------------------

/// A single completed step record within a stage history entry.
#[derive(Debug, Serialize)]
pub struct FlowStepRecord {
    pub step: String,
    pub status: String,
    pub step_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// A stage record within the history output.
#[derive(Debug, Serialize)]
pub struct FlowStageRecord {
    pub stage: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub force_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_reason: Option<String>,
    pub steps: Vec<FlowStepRecord>,
}

/// Output payload for `awf flow history`.
#[derive(Debug, Serialize)]
pub struct FlowHistoryOutput {
    pub entries: Vec<FlowStageRecord>,
}

impl fmt::Display for FlowHistoryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for entry in &self.entries {
            writeln!(
                f,
                "• stage {} started at {}",
                entry.stage.magenta(),
                pretty_dt(&entry.started_at).bright_black(),
            )?;
            for step in &entry.steps {
                let (icon, verb, name_colored) = match step.status.as_str() {
                    "pass" => (
                        "✓".green().to_string(),
                        "completed at",
                        step.step.green().to_string(),
                    ),
                    "fail" => (
                        "✗".red().to_string(),
                        "failed at",
                        step.step.red().to_string(),
                    ),
                    _ => ("?".to_string(), "at", step.step.blue().to_string()),
                };
                match (&step.completed_at, step.duration_ms) {
                    (Some(ts), Some(ms)) => writeln!(
                        f,
                        "    {icon} {name_colored} step {verb} {}",
                        format!("{} ({} ms)", pretty_dt(ts), ms).bright_black(),
                    )?,
                    (Some(ts), None) => writeln!(
                        f,
                        "    {icon} {name_colored} step {verb} {}",
                        pretty_dt(ts).bright_black(),
                    )?,
                    _ => {
                        writeln!(f, "    {icon} {name_colored} step {verb} ?")?;
                    }
                }
            }
            if let Some(ref completed_at) = entry.completed_at {
                if entry.force_completed {
                    let icon = "⚠".yellow();
                    writeln!(
                        f,
                        "    {icon} stage {} at {}",
                        "force completed".yellow(),
                        pretty_dt(completed_at).bright_black(),
                    )?;
                    if let Some(ref reason) = entry.force_reason {
                        writeln!(f, "        {}", reason.yellow())?;
                    }
                } else {
                    let icon = "✓".green();
                    writeln!(
                        f,
                        "    {icon} stage completed at {}",
                        pretty_dt(completed_at).bright_black(),
                    )?;
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow archive`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow archive`.
#[derive(Debug, Serialize)]
pub struct FlowArchiveOutput {
    pub archived_flow: String,
}

impl fmt::Display for FlowArchiveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Archived flow: {}", self.archived_flow.cyan())
    }
}

// ---------------------------------------------------------------------------
// `awf flow new [--flow <path>] --workflow <name>`
// ---------------------------------------------------------------------------

/// Handles `awf flow new [--flow <path>] --workflow <workflow>`.
///
/// Resolves the flow path, loads the workflow to find its first stage, and
/// creates a new flow at the path.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if the workflow cannot be
/// loaded.
pub fn new(
    flow_arg: &str,
    workflow: &str,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let wf = super::load_workflow_or_exit(&entries, workflow)?;
    super::validate_workflow_or_exit(&wf, &entries, &config.commands);

    let flow_dir = resolve_new_flow_dir(flow_arg, &wf, &project_root);

    let first_stage = wf.stages.first().map_or_else(
        || {
            eprint_error(format_args!(
                "workflow {workflow:?} has no stages; cannot create a flow"
            ));
            process::exit(1);
        },
        |s| s.name.as_str(),
    );

    match fs::create_flow(&flow_dir, workflow, first_stage) {
        Ok(()) => {
            let output = FlowNewOutput {
                path: flow_dir.display().to_string(),
                workflow: workflow.to_string(),
                stage: first_stage.to_string(),
            };
            print_output(&output, fmt);
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
) -> PathBuf {
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

    let base = expand_tilde(default_dir);
    let base = if base.is_relative() { project_root.join(base) } else { base };

    base.join(flow_arg)
}

// ---------------------------------------------------------------------------
// `awf flow list [--all]`
// ---------------------------------------------------------------------------

/// Handles `awf flow list [--all]`.
///
/// Scans `config.flow_search_dirs` for flows. Without `--all`, archived flows
/// are excluded.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no project root can be
/// found.
pub fn list(
    include_archived: bool,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;

    let all_flows = fs::scan_flows(&config.flow_search_dirs)?;
    let flows: Vec<_> = if include_archived {
        all_flows
    } else {
        all_flows.into_iter().filter(|f| !f.archived).collect()
    };

    let output = FlowListOutput {
        flows: flows
            .into_iter()
            .map(|f| FlowListEntry {
                path: f.path.display().to_string(),
                workflow: f.workflow,
                stage: f.current_stage,
                archived: f.archived,
            })
            .collect(),
        include_archived,
    };
    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow status [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow status [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if the workflow/flow cannot
/// be loaded.
pub fn status(
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = super::load_workflow_or_exit(&entries, &flow.workflow)?;

    let current_stage =
        workflow.stages.iter().find(|s| s.name == flow.current_stage);

    let steps = match current_stage {
        Some(stage) => {
            let current_stage_entries: Vec<_> = flow
                .step_log
                .iter()
                .filter(|e| e.stage == flow.current_stage)
                .collect();

            stage
                .steps
                .iter()
                .map(|step| {
                    let status_str = current_stage_entries
                        .iter()
                        .rfind(|e| e.step == step.name)
                        .map_or_else(
                            || "pending".to_string(),
                            |entry| match entry.status {
                                StepStatus::Pass => "pass".to_string(),
                                StepStatus::Fail => "fail".to_string(),
                            },
                        );
                    StepStatusLine {
                        name: step.name.clone(),
                        status: status_str,
                    }
                })
                .collect()
        }
        None => vec![],
    };

    let output = FlowStatusOutput {
        flow: flow.id.clone(),
        stage: flow.current_stage.clone(),
        steps,
    };

    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow next-step [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow next-step [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn next(
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = super::load_workflow_or_exit(&entries, &flow.workflow)?;
    super::validate_workflow_or_exit(&workflow, &entries, &config.commands);
    let artifacts = collect_artifacts(&flow_dir);

    let output = match next_action(&workflow, &flow, &artifacts) {
        NextAction::RunStep(step) => FlowNextOutput {
            action: "run_step".to_string(),
            step: Some(step.name.clone()),
            artifact: None,
            next_stage: None,
            prompt: None,
            message: format!(
                "[command] {} \u{2014} next step to run: awf flow run {}",
                step.name, step.name
            ),
        },
        NextAction::CompleteAgentStep(step) => {
            let prompt =
                if let StepKind::Agent { prompt: Some(ref filename), .. } =
                    step.kind
                {
                    match fs::load_prompt_file(
                        &entries,
                        &flow.workflow,
                        filename,
                    ) {
                        Ok(contents) => Some(contents),
                        Err(e) => {
                            eprint_warning(format_args!(
                                "could not load prompt file {filename:?}: {e}"
                            ));
                            None
                        }
                    }
                } else {
                    None
                };
            FlowNextOutput {
                action: "complete_agent_step".to_string(),
                step: Some(step.name.clone()),
                artifact: None,
                next_stage: None,
                prompt,
                message: format!(
                    "Mark complete with: awf flow step done {}",
                    step.name
                ),
            }
        }
        NextAction::ProduceArtifact(name) => FlowNextOutput {
            action: "produce_artifact".to_string(),
            step: None,
            artifact: Some(name.clone()),
            next_stage: None,
            prompt: None,
            message: format!(
                "All steps complete. Produce artifact: {name}\n\
                 Write it to the artifacts/ directory inside the flow path."
            ),
        },
        NextAction::CanAdvance => {
            let next_stage_name = workflow
                .stages
                .iter()
                .position(|s| s.name == flow.current_stage)
                .and_then(|idx| workflow.stages.get(idx + 1))
                .map_or_else(|| "next".to_string(), |s| s.name.clone());
            FlowNextOutput {
                action: "can_advance".to_string(),
                step: None,
                artifact: None,
                next_stage: Some(next_stage_name.clone()),
                prompt: None,
                message: format!(
                    "All steps complete. Run `awf flow next-stage` to move to \
                     {next_stage_name}."
                ),
            }
        }
        NextAction::Done => FlowNextOutput {
            action: "done".to_string(),
            step: None,
            artifact: None,
            next_stage: None,
            prompt: None,
            message: "All stages complete. The workflow is done.".to_string(),
        },
    };

    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow run <step> [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow run <step> [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn run(
    step_name: &str,
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = super::load_workflow_or_exit(&entries, &flow.workflow)?;

    let step = find_step_in_current_stage(&workflow, &flow, step_name)
        .unwrap_or_else(|| {
            eprint_error(format_args!(
                "step {step_name:?} not found in current stage {:?}",
                flow.current_stage
            ));
            process::exit(1);
        });

    if matches!(step.kind, StepKind::Agent { .. }) {
        eprint_error(format_args!(
            "Step {step_name:?} is an agent step. Use \
             `awf flow step done {step_name}` instead."
        ));
        process::exit(1);
    }

    let shell_cmd = resolve_shell_command(step, &workflow, &config.commands)
        .unwrap_or_else(|| {
            eprint_error(format_args!(
                "step {step_name:?} has no resolvable shell command"
            ));
            process::exit(1);
        });

    let started_at = Utc::now();
    let result = run_command(&shell_cmd)?;

    let stage = flow.current_stage.clone();
    let flow = if result.exit_code == 0 {
        record_step_pass(
            flow,
            step,
            &stage,
            Some(result.exit_code),
            Some(started_at),
            Some(result.duration_ms),
        )
    } else {
        record_step_fail(
            flow,
            step,
            &stage,
            Some(result.exit_code),
            Some(started_at),
            Some(result.duration_ms),
        )
    };

    fs::save_flow(&flow_dir, &flow)?;

    let status = if result.exit_code == 0 { "pass" } else { "fail" };
    let output = FlowRunOutput {
        step: step_name.to_string(),
        command: shell_cmd,
        status: status.to_string(),
        duration_ms: result.duration_ms,
    };
    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow step done <step> [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow step done <step> [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn step_done(
    step_name: &str,
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = super::load_workflow_or_exit(&entries, &flow.workflow)?;

    let step = find_step_in_current_stage(&workflow, &flow, step_name)
        .unwrap_or_else(|| {
            eprint_error(format_args!(
                "step {step_name:?} not found in current stage {:?}",
                flow.current_stage
            ));
            process::exit(1);
        });

    if matches!(step.kind, StepKind::Command { .. }) {
        eprint_error(format_args!(
            "Step {step_name:?} is a command step. Use \
             `awf flow run {step_name}` instead."
        ));
        process::exit(1);
    }

    let stage = flow.current_stage.clone();
    let flow = record_step_pass(flow, step, &stage, None, None, None);

    fs::save_flow(&flow_dir, &flow)?;

    let output = FlowStepDoneOutput { step: step_name.to_string() };
    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow validate [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow validate [--flow <path>]`.
///
/// Checks that:
/// - All artifacts declared as `produces` by completed stages exist on disk.
/// - No artifacts exist that are not declared by any completed or current
///   stage.
///
/// # Errors
///
/// Returns an error if validation fails or for unexpected I/O failures.
pub fn validate(
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = super::load_workflow_or_exit(&entries, &flow.workflow)?;

    let completed_stage_names: HashSet<&str> = flow
        .stage_history
        .iter()
        .filter(|e| e.completed_at.is_some())
        .map(|e| e.stage.as_str())
        .collect();

    let mut expected: HashSet<String> = HashSet::new();
    let mut valid: HashSet<String> = HashSet::new();

    for stage in &workflow.stages {
        let is_completed = completed_stage_names.contains(stage.name.as_str());
        let is_current = stage.name == flow.current_stage;

        if is_completed {
            expected.extend(stage.produces.iter().cloned());
            valid.extend(stage.produces.iter().cloned());
        } else if is_current {
            valid.extend(stage.produces.iter().cloned());
        }
    }

    let actual = collect_artifacts(&flow_dir);
    let missing: Vec<&str> = expected
        .iter()
        .filter(|a| !actual.contains(a.as_str()))
        .map(String::as_str)
        .collect();
    let redundant: Vec<&str> = actual
        .iter()
        .filter(|a| !valid.contains(a.as_str()))
        .map(String::as_str)
        .collect();

    if missing.is_empty() && redundant.is_empty() {
        let output = FlowValidateOutput { valid: true };
        print_output(&output, fmt);
        return Ok(());
    }

    for name in &missing {
        eprint_error(format_args!("missing artifact: {name}"));
    }
    for name in &redundant {
        eprint_warning(format_args!("redundant artifact: {name}"));
    }

    color_eyre::eyre::bail!("flow artifact validation failed");
}

// ---------------------------------------------------------------------------
// `awf flow next-stage [--force] [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow next-stage [--force] [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn advance(
    flow_flag: Option<&str>,
    force: Option<String>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = super::load_workflow_or_exit(&entries, &flow.workflow)?;
    super::validate_workflow_or_exit(&workflow, &entries, &config.commands);
    let artifacts = collect_artifacts(&flow_dir);

    // --force bypasses step gates but never artifact gates.
    #[allow(clippy::single_match_else, clippy::option_if_let_else)]
    let force_reason = match force {
        Some(reason) => {
            check_produces_or_exit(&workflow, &flow, &artifacts);
            Some(reason)
        }
        None => {
            if let Err(blocker) = can_advance(&workflow, &flow, &artifacts) {
                match blocker {
                    AdvanceBlocker::StepFailed(step) => {
                        eprint_error(format_args!(
                            "`{step}` failed. Fix the error then run \
                             `awf flow run {step}` before advancing."
                        ));
                    }
                    AdvanceBlocker::StepIncomplete(step) => {
                        eprint_error(format_args!(
                            "`{step}` is not complete. Complete it before \
                             advancing."
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
                process::exit(1);
            }
            None
        }
    };

    let next_stage_name = {
        let current_idx = workflow
            .stages
            .iter()
            .position(|s| s.name == flow.current_stage)
            .unwrap_or_else(|| {
                eprint_error(format_args!(
                    "current stage {:?} not found in workflow",
                    flow.current_stage
                ));
                process::exit(1);
            });

        let Some(next) = workflow.stages.get(current_idx + 1) else {
            eprint_error("already on the last stage; nothing to advance to");
            process::exit(1);
        };

        next.name.clone()
    };

    match can_enter_stage(&workflow, &flow, &next_stage_name, &artifacts) {
        EnterResult::Ok => {}
        EnterResult::MissingRequires(missing) => {
            eprint_error(format_args!(
                "Cannot enter stage `{next_stage_name}`. Missing \
                 required artifacts: {}",
                missing.join(", ")
            ));
            process::exit(1);
        }
        EnterResult::AlreadyCompleted { produces } => {
            eprint_warning(format_args!(
                "Stage `{next_stage_name}` has already been completed \
                 (produces: {}). Re-entering it will overwrite previous \
                 results.",
                produces.join(", ")
            ));
        }
    }

    let updated_flow = match advance_stage(&workflow, flow, force_reason) {
        Ok(f) => f,
        Err(Error::AlreadyOnLastStage) => {
            eprint_error("already on the last stage; nothing to advance to");
            process::exit(1);
        }
        Err(e) => return Err(e.into()),
    };

    let new_stage = updated_flow.current_stage.clone();
    fs::save_flow(&flow_dir, &updated_flow)?;

    let is_final = workflow.stages.last().is_some_and(|s| s.name == new_stage);
    let output = FlowAdvanceOutput { stage: new_stage, is_final };
    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow history [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow history [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn history(
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    let flow = load_flow_or_exit(&flow_dir)?;

    let mut entries: Vec<FlowStageRecord> = flow
        .stage_history
        .iter()
        .map(|entry| FlowStageRecord {
            stage: entry.stage.clone(),
            started_at: entry
                .started_at
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            completed_at: entry
                .completed_at
                .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            force_completed: entry.force_completed,
            force_reason: entry.force_reason.clone(),
            steps: build_step_records(&flow, &entry.stage),
        })
        .collect();

    let current_already_shown =
        flow.stage_history.iter().any(|e| e.stage == flow.current_stage);

    if !current_already_shown {
        entries.push(FlowStageRecord {
            stage: flow.current_stage.clone(),
            started_at: flow
                .stage_started_at
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            completed_at: None,
            force_completed: false,
            force_reason: None,
            steps: build_step_records(&flow, &flow.current_stage),
        });
    }

    let output = FlowHistoryOutput { entries };
    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow archive [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow archive [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn archive(
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    match fs::archive_flow(&flow_dir) {
        Ok(()) => {
            let output = FlowArchiveOutput {
                archived_flow: flow_dir.display().to_string(),
            };
            print_output(&output, fmt);
            Ok(())
        }
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

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Builds `FlowStepRecord` entries for all steps in `stage_name` from the
/// flow's step log.
fn build_step_records(
    flow: &FlowState,
    stage_name: &str,
) -> Vec<FlowStepRecord> {
    flow.step_log
        .iter()
        .filter(|e| e.stage == stage_name)
        .map(|entry| {
            let status_str = match entry.status {
                StepStatus::Pass => "pass",
                StepStatus::Fail => "fail",
            };
            let completed_at = entry
                .completed_at
                .map(|ts| ts.format("%Y-%m-%dT%H:%M:%SZ").to_string());
            let duration_ms = if entry.step_type == "command" {
                entry.duration_ms
            } else {
                None
            };
            FlowStepRecord {
                step: entry.step.clone(),
                status: status_str.to_string(),
                step_type: entry.step_type.clone(),
                completed_at,
                duration_ms,
            }
        })
        .collect()
}

/// Parses an ISO 8601 timestamp and formats it in the local timezone as
/// `"YYYY-MM-DD HH:MM:SS ±HH:MM"`. Returns the input unchanged if it cannot
/// be parsed (e.g. `"123ms"` duration strings).
fn pretty_dt(s: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(s).map_or_else(
        |_| s.to_owned(),
        |dt| {
            dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S %:z").to_string()
        },
    )
}

/// Checks that all `produces` artifacts for the current stage exist, exiting
/// with an error if any are missing. Used by `--force` which bypasses step
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
            process::exit(1);
        }
    }
}

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
                "no flow specified; pass --flow <path> or set AWF_ACTIVE_FLOW",
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

/// Finds a step by name in `flow.current_stage`.
fn find_step_in_current_stage<'a>(
    workflow: &'a Workflow,
    flow: &FlowState,
    step_name: &str,
) -> Option<&'a Step> {
    let stage =
        workflow.stages.iter().find(|s| s.name == flow.current_stage)?;
    stage.steps.iter().find(|s| s.name == step_name)
}

/// Resolves the shell command string for a command step.
///
/// For `command` alias steps, looks up the alias in `workflow.commands` first,
/// then falls back to `repo_commands` (from `.config/awf.toml`). This allows
/// common commands like `test` and `format` to be defined once at the repo
/// level rather than repeated in every workflow.
fn resolve_shell_command(
    step: &Step,
    workflow: &Workflow,
    repo_commands: &HashMap<String, String>,
) -> Option<String> {
    match &step.kind {
        StepKind::Command { command: Some(alias), run: None } => workflow
            .commands
            .get(alias.as_str())
            .or_else(|| repo_commands.get(alias.as_str()))
            .cloned(),
        StepKind::Command { run: Some(inline), command: None } => {
            Some(inline.clone())
        }
        _ => None,
    }
}
