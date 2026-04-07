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
use crate::engine::state::advance_stage;
use crate::engine::state::record_stage_fail;
use crate::engine::state::record_stage_pass;
use crate::engine::state::skip_stage;
use crate::model::flow::FlowState;
use crate::model::flow::RunStatus;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
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

/// The complete status payload for `awf flow status`.
#[derive(Debug, Serialize)]
pub struct FlowStatusOutput {
    pub flow: String,
    pub stage: String,
    pub stage_kind: String,
    pub status: String,
}

impl fmt::Display for FlowStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Flow:  {}", self.flow.cyan())?;
        writeln!(f, "Stage:  {}", self.stage.magenta())?;
        let badge = match self.status.as_str() {
            "pass" => "[✓ pass]".green().to_string(),
            "fail" => "[✗ fail]".red().to_string(),
            _ => "[· pending]".bright_black().to_string(),
        };
        writeln!(f, "Status: {badge}")
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow next-step`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow next-step`.
#[derive(Debug, Serialize)]
pub struct FlowNextOutput {
    /// Discriminator: `"run_command"`, `"complete_agent"`,
    /// `"produce_artifact"`, `"can_advance"`, or `"done"`.
    pub action: String,
    /// The stage name (for `run_command` and `complete_agent`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    /// The artifact name (for `produce_artifact`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    /// The name of the next stage (for `can_advance`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_stage: Option<String>,
    /// The raw prompt file contents (for `complete_agent`, when available).
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
    pub stage: String,
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
                "Fix the error, then run: {}",
                "awf flow run".magenta()
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow done`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow done`.
#[derive(Debug, Serialize)]
pub struct FlowDoneOutput {
    pub stage: String,
}

impl fmt::Display for FlowDoneOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} Stage {} marked complete.",
            "✓".green(),
            self.stage.magenta()
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

/// A single run record within a stage history entry.
#[derive(Debug, Serialize)]
pub struct FlowRunRecord {
    pub stage_kind: String,
    pub status: String,
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
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
    pub runs: Vec<FlowRunRecord>,
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
            for run in &entry.runs {
                let (icon, verb) = match run.status.as_str() {
                    "pass" => ("✓".green().to_string(), "completed at"),
                    "fail" => ("✗".red().to_string(), "failed at"),
                    _ => ("?".to_string(), "at"),
                };
                let name_colored = match run.status.as_str() {
                    "pass" => entry.stage.green().to_string(),
                    "fail" => entry.stage.red().to_string(),
                    _ => entry.stage.blue().to_string(),
                };
                match (&run.completed_at, run.duration_ms) {
                    (Some(ts), Some(ms)) => writeln!(
                        f,
                        "    {icon} {name_colored} {verb} {}",
                        format!("{} ({} ms)", pretty_dt(ts), ms).bright_black(),
                    )?,
                    (Some(ts), None) => writeln!(
                        f,
                        "    {icon} {name_colored} {verb} {}",
                        pretty_dt(ts).bright_black(),
                    )?,
                    _ => {
                        writeln!(f, "    {icon} {name_colored} {verb} ?")?;
                    }
                }
            }
            if let Some(ref completed_at) = entry.completed_at {
                if entry.skipped {
                    let icon = "⏭".yellow();
                    writeln!(
                        f,
                        "    {icon} stage {} at {}",
                        "skipped".yellow(),
                        pretty_dt(completed_at).bright_black(),
                    )?;
                } else if entry.force_completed {
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

    let (status_str, kind_str) = match current_stage {
        Some(stage) => {
            let status = flow
                .stage_log
                .iter()
                .rfind(|e| e.stage == flow.current_stage)
                .map_or_else(
                    || "pending".to_string(),
                    |entry| match entry.status {
                        RunStatus::Pass => "pass".to_string(),
                        RunStatus::Fail => "fail".to_string(),
                    },
                );
            let kind = match &stage.kind {
                StageKind::Command { .. } => "command",
                StageKind::Agent { .. } => "agent",
            };
            (status, kind.to_string())
        }
        None => ("pending".to_string(), "unknown".to_string()),
    };

    let output = FlowStatusOutput {
        flow: flow.id.clone(),
        stage: flow.current_stage.clone(),
        stage_kind: kind_str,
        status: status_str,
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
        NextAction::RunCommand(stage) => FlowNextOutput {
            action: "run_command".to_string(),
            stage: Some(stage.name.clone()),
            artifact: None,
            next_stage: None,
            prompt: None,
            message: format!(
                "[command] {} \u{2014} run: awf flow run",
                stage.name
            ),
        },
        NextAction::CompleteAgent(stage) => {
            let prompt =
                if let StageKind::Agent { prompt: Some(ref filename), .. } =
                    stage.kind
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
                action: "complete_agent".to_string(),
                stage: Some(stage.name.clone()),
                artifact: None,
                next_stage: None,
                prompt,
                message: "Mark complete with: awf flow done".to_string(),
            }
        }
        NextAction::ProduceArtifact(name) => FlowNextOutput {
            action: "produce_artifact".to_string(),
            stage: None,
            artifact: Some(name.clone()),
            next_stage: None,
            prompt: None,
            message: format!(
                "Stage complete. Produce artifact: {name}\n\
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
                stage: None,
                artifact: None,
                next_stage: Some(next_stage_name.clone()),
                prompt: None,
                message: format!(
                    "Stage complete. Run `awf flow next-stage` to move to \
                     {next_stage_name}."
                ),
            }
        }
        NextAction::Done => FlowNextOutput {
            action: "done".to_string(),
            stage: None,
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
// `awf flow run [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow run [--flow <path>]`.
///
/// Runs the current stage if it is a command stage.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn run(
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

    let stage = current_stage_or_exit(&workflow, &flow);

    if matches!(stage.kind, StageKind::Agent { .. }) {
        eprint_error(format_args!(
            "Stage {:?} is an agent stage. Use `awf flow done` instead.",
            stage.name
        ));
        process::exit(1);
    }

    let shell_cmd = resolve_shell_command(stage, &workflow, &config.commands)
        .unwrap_or_else(|| {
            eprint_error(format_args!(
                "stage {:?} has no resolvable shell command",
                stage.name
            ));
            process::exit(1);
        });

    let started_at = Utc::now();
    let result = run_command(&shell_cmd)?;

    let flow = if result.exit_code == 0 {
        record_stage_pass(
            flow,
            stage,
            Some(result.exit_code),
            Some(started_at),
            Some(result.duration_ms),
        )
    } else {
        record_stage_fail(
            flow,
            stage,
            Some(result.exit_code),
            Some(started_at),
            Some(result.duration_ms),
        )
    };

    fs::save_flow(&flow_dir, &flow)?;

    let status = if result.exit_code == 0 { "pass" } else { "fail" };
    let output = FlowRunOutput {
        stage: stage.name.clone(),
        command: shell_cmd,
        status: status.to_string(),
        duration_ms: result.duration_ms,
    };
    print_output(&output, fmt);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow done [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow done [--flow <path>]`.
///
/// Marks the current agent stage as complete.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn done(
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

    let stage = current_stage_or_exit(&workflow, &flow);

    if matches!(stage.kind, StageKind::Command { .. }) {
        eprint_error(format_args!(
            "Stage {:?} is a command stage. Use `awf flow run` instead.",
            stage.name
        ));
        process::exit(1);
    }

    let flow = record_stage_pass(flow, stage, None, None, None);

    fs::save_flow(&flow_dir, &flow)?;

    let output = FlowDoneOutput { stage: stage.name.clone() };
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
// `awf flow next-stage [--force] [--skip] [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow next-stage [--force] [--skip] [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn advance(
    flow_flag: Option<&str>,
    force: Option<String>,
    skip: bool,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    if skip && force.is_some() {
        eprint_error("--skip and --force cannot be used together");
        process::exit(1);
    }

    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = super::load_workflow_or_exit(&entries, &flow.workflow)?;
    super::validate_workflow_or_exit(&workflow, &entries, &config.commands);

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
                process::exit(1);
            }
            Err(Error::AlreadyOnLastStage) => {
                eprint_error(
                    "already on the last stage; nothing to advance to",
                );
                process::exit(1);
            }
            Err(e) => return Err(e.into()),
        }
    } else {
        let artifacts = collect_artifacts(&flow_dir);

        // --force bypasses stage gates but never artifact gates.
        #[allow(clippy::single_match_else, clippy::option_if_let_else)]
        let force_reason = match force {
            Some(reason) => {
                check_produces_or_exit(&workflow, &flow, &artifacts);
                Some(reason)
            }
            None => {
                if let Err(blocker) = can_advance(&workflow, &flow, &artifacts)
                {
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
                    process::exit(1);
                }
                None
            }
        };

        match advance_stage(&workflow, flow, force_reason) {
            Ok(f) => f,
            Err(Error::AlreadyOnLastStage) => {
                eprint_error(
                    "already on the last stage; nothing to advance to",
                );
                process::exit(1);
            }
            Err(e) => return Err(e.into()),
        }
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
            skipped: entry.skipped,
            runs: build_stage_runs(&flow, &entry.stage),
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
            skipped: false,
            runs: build_stage_runs(&flow, &flow.current_stage),
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

/// Builds `FlowRunRecord` entries for all runs of `stage_name` from the
/// flow's stage log.
fn build_stage_runs(flow: &FlowState, stage_name: &str) -> Vec<FlowRunRecord> {
    flow.stage_log
        .iter()
        .filter(|e| e.stage == stage_name)
        .map(|entry| {
            let status_str = match entry.status {
                RunStatus::Pass => "pass",
                RunStatus::Fail => "fail",
            };
            let completed_at = entry
                .completed_at
                .map(|ts| ts.format("%Y-%m-%dT%H:%M:%SZ").to_string());
            let duration_ms = if entry.stage_kind == "command" {
                entry.duration_ms
            } else {
                None
            };
            FlowRunRecord {
                stage_kind: entry.stage_kind.clone(),
                status: status_str.to_string(),
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

/// Resolves the shell command string for a command stage.
///
/// For `command` alias stages, looks up the alias in `workflow.commands` first,
/// then falls back to `repo_commands` (from `.config/awf.toml`). This allows
/// common commands like `test` and `format` to be defined once at the repo
/// level rather than repeated in every workflow.
fn resolve_shell_command(
    stage: &Stage,
    workflow: &Workflow,
    repo_commands: &HashMap<String, String>,
) -> Option<String> {
    match &stage.kind {
        StageKind::Command { command: Some(alias), run: None } => workflow
            .commands
            .get(alias.as_str())
            .or_else(|| repo_commands.get(alias.as_str()))
            .cloned(),
        StageKind::Command { run: Some(inline), command: None } => {
            Some(inline.clone())
        }
        _ => None,
    }
}
