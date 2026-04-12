//! Handlers for `awf flow` subcommands.

use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use chrono::Local;
use owo_colors::OwoColorize as _;

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
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::store::fs;
use crate::store::resolve::resolve_flow_path;

// ---------------------------------------------------------------------------
// Output types — `awf flow new`
// ---------------------------------------------------------------------------

/// The prompt payload for an agent stage.
#[derive(Debug)]
pub enum AgentPrompt {
    /// Inline instruction string passed directly to the agent.
    Inline(String),
    /// Resolved absolute path to a prompt file.
    File(PathBuf),
}

/// Per-stage data for the agent guidance section of `awf flow new`.
#[derive(Debug)]
pub struct FlowNewStage {
    pub name: String,
    pub description: Option<String>,
    /// Inline agent prompt text (for stage-listing previews of non-current
    /// stages). `None` for command stages or `prompt_file` stages.
    pub prompt: Option<String>,
    pub skippable: bool,
    pub produces: Vec<String>,
    pub is_current: bool,
}

/// What the agent should do next for the first stage.
#[derive(Debug)]
pub enum FirstStageAction {
    AgentComplete { prompt: Option<AgentPrompt> },
    RunCommand,
}

/// Output payload for `awf flow new`.
#[derive(Debug)]
pub struct FlowNewOutput {
    pub path: String,
    pub workflow: String,
    pub stage: String,
    pub stages: Vec<FlowNewStage>,
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
        writeln!(
            f,
            "From now on, \"active flow\" refers to `{}`.",
            self.path.cyan()
        )?;
        writeln!(f)?;

        writeln!(
            f,
            "Workflow `{}` has {} stages:",
            self.workflow.blue(),
            self.stages.len()
        )?;
        for (i, stage) in self.stages.iter().enumerate() {
            let suffix = if stage.is_current {
                " (current)"
            } else if stage.skippable {
                " (skippable)"
            } else {
                ""
            };
            writeln!(f, "{}. {}{}", i + 1, stage.name.magenta(), suffix)?;
            // Current stage: show description (prompt appears in "Next up"
            // section). Other stages: show inline prompt as a preview, falling
            // back to description.
            let blurb = if stage.is_current {
                stage.description.as_deref()
            } else {
                stage.prompt.as_deref().or(stage.description.as_deref())
            };
            if let Some(text) = blurb {
                writeln!(f, "    - {text}")?;
            }
            for artifact in &stage.produces {
                writeln!(f, "    - Produces: {artifact}")?;
            }
        }

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
// Output types — `awf flow list`
// ---------------------------------------------------------------------------

/// A single flow entry in the list output.
#[derive(Debug)]
pub struct FlowListEntry {
    pub path: String,
    pub workflow: String,
    pub stage: String,
    pub archived: bool,
}

/// Output payload for `awf flow list`.
#[derive(Debug)]
pub struct FlowListOutput {
    pub flows: Vec<FlowListEntry>,
    /// Controls the empty-list hint and whether the ARCHIVED column is shown.
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
                 --flow=<path> --workflow=<workflow>"
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
#[derive(Debug)]
pub struct FlowStatusOutput {
    pub flow: String,
    pub stage: String,
    pub status: String,
}

impl fmt::Display for FlowStatusOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  flow: {}", self.flow.cyan())?;
        writeln!(f, " stage: {}", self.stage.magenta())?;
        let badge = match self.status.as_str() {
            "archived" => "[· archived]".bright_black().to_string(),
            _ => "[● active]".green().to_string(),
        };
        writeln!(f, "status: {badge}")
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow next-step`
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
// Output types — `awf flow run`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow run`.
#[derive(Debug)]
pub struct FlowRunOutput {
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

// ---------------------------------------------------------------------------
// Output types — `awf flow validate`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow validate` (success path).
#[derive(Debug)]
pub struct FlowValidateOutput;

impl fmt::Display for FlowValidateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "✓ Flow artifacts valid.".green())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow next-stage`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow next-stage`.
#[derive(Debug)]
pub struct FlowAdvanceOutput {
    pub stage: String,
    pub is_final: bool,
    pub flow_path: String,
    pub next_action: FirstStageAction,
}

impl fmt::Display for FlowAdvanceOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {}", "▶ Entered stage:".green(), self.stage.magenta())?;
        if self.is_final {
            writeln!(f, "{}", "  This is the final stage.".bright_black())?;
        }
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
// Output types — `awf flow history`
// ---------------------------------------------------------------------------

/// A single entry in the history output, mirroring [`StageHistoryEntry`] with
/// timestamps serialized as ISO 8601 strings.
#[derive(Debug)]
pub struct FlowHistoryEntry {
    pub stage: String,
    pub time: String,
    pub status: String,
    pub force_reason: Option<String>,
}

/// Output payload for `awf flow history`.
#[derive(Debug)]
pub struct FlowHistoryOutput {
    pub flow_path: String,
    pub flow_started_at: String,
    pub entries: Vec<FlowHistoryEntry>,
}

impl fmt::Display for FlowHistoryOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} flow started at {}",
            self.flow_path.cyan(),
            pretty_dt(&self.flow_started_at).bright_black(),
        )?;
        for entry in &self.entries {
            match entry.status.as_str() {
                "completed" => writeln!(
                    f,
                    "    {} {} completed at {}",
                    "✓".green(),
                    entry.stage.green(),
                    pretty_dt(&entry.time).bright_black(),
                )?,
                "failed" => writeln!(
                    f,
                    "    {} {} failed at {}",
                    "✗".red(),
                    entry.stage.red(),
                    pretty_dt(&entry.time).bright_black(),
                )?,
                "force_completed" => {
                    writeln!(
                        f,
                        "    {} {} force completed at {}",
                        "⚠".yellow(),
                        entry.stage.yellow(),
                        pretty_dt(&entry.time).bright_black(),
                    )?;
                    if let Some(ref reason) = entry.force_reason {
                        writeln!(f, "        {}", reason.bright_black())?;
                    }
                }
                "skipped" => writeln!(
                    f,
                    "    {} {} skipped at {}",
                    "⏭".yellow(),
                    entry.stage.yellow(),
                    pretty_dt(&entry.time).bright_black(),
                )?,
                _ => {}
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow archive`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow archive`.
#[derive(Debug)]
pub struct FlowArchiveOutput {
    pub archived_flow: String,
}

impl fmt::Display for FlowArchiveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Archived flow: {}", self.archived_flow.cyan())
    }
}

// ---------------------------------------------------------------------------
// `awf flow new [--flow=<path>] --workflow=<name>`
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

    let wf_dir = fs::find_workflow_dir(&entries, workflow).unwrap_or_default();

    match fs::create_flow(&flow_dir, workflow, first_stage) {
        Ok(()) => {
            let stages = wf
                .stages
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let prompt = match &s.kind {
                        StageKind::Agent { prompt: Some(text), .. } => {
                            Some(text.clone())
                        }
                        _ => None,
                    };
                    FlowNewStage {
                        name: s.name.clone(),
                        description: s.description.clone(),
                        prompt,
                        skippable: s.skippable,
                        produces: s.produces.clone(),
                        is_current: i == 0,
                    }
                })
                .collect();
            let first_stage_action = match wf.stages.first().map(|s| &s.kind) {
                Some(StageKind::Agent { prompt, prompt_file }) => {
                    FirstStageAction::AgentComplete {
                        prompt: build_agent_prompt_from_fields(
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
pub fn list(include_archived: bool) -> color_eyre::Result<()> {
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
    print_output(&output);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow status [--flow=<path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow status [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if the workflow/flow cannot
/// be loaded.
pub fn status(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let flow = load_flow_or_exit(&flow_dir)?;

    let status_str = if flow.archived { "archived" } else { "active" };

    let output = FlowStatusOutput {
        flow: flow.id.clone(),
        stage: flow.current_stage,
        status: status_str.to_string(),
    };

    print_output(&output);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow next-step [--flow=<path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow next-step [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn next(flow_flag: Option<&str>) -> color_eyre::Result<()> {
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

// ---------------------------------------------------------------------------
// `awf flow run [--flow=<path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow run [--flow=<path>]`.
///
/// Runs the current stage if it is a command stage.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn run(flow_flag: Option<&str>) -> color_eyre::Result<()> {
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
            "Stage {:?} is an agent stage. Use `awf flow next-stage` instead.",
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

    let result = run_command(&shell_cmd)?;

    let flow = if result.exit_code == 0 {
        record_stage_pass(flow, stage, Some(result.duration_ms))
    } else {
        record_stage_fail(flow, stage, Some(result.duration_ms))
    };

    fs::save_flow(&flow_dir, &flow)?;

    let status = if result.exit_code == 0 { "pass" } else { "fail" };
    let output = FlowRunOutput {
        command: shell_cmd,
        status: status.to_string(),
        duration_ms: result.duration_ms,
    };
    print_output(&output);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow validate [--flow=<path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow validate [--flow=<path>]`.
///
/// Checks that:
/// - All artifacts declared as `produces` by completed stages exist on disk.
/// - No artifacts exist that are not declared by any completed or current
///   stage.
///
/// # Errors
///
/// Returns an error if validation fails or for unexpected I/O failures.
pub fn validate(flow_flag: Option<&str>) -> color_eyre::Result<()> {
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
        .filter(|e| {
            matches!(
                e.status,
                crate::model::flow::StageStatus::Completed
                    | crate::model::flow::StageStatus::ForceCompleted
                    | crate::model::flow::StageStatus::Skipped
            )
        })
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
        let output = FlowValidateOutput;
        print_output(&output);
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
// `awf flow next-stage [--force] [--skip] [--flow=<path>]`
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

    // For agent stages (and not --skip): auto-record completion before
    // advancing. This replaces the former `complete-stage` command.
    // The completion is saved to disk immediately so that if the advance
    // fails (e.g. missing artifact) the stage is still recorded as done and
    // `next-step` correctly reports ProduceArtifact on the next call.
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
                process::exit(1);
            }
            Err(e) => return Err(e.into()),
        }
    };

    let new_stage = updated_flow.current_stage.clone();
    fs::save_flow(&flow_dir, &updated_flow)?;

    let wf_dir =
        fs::find_workflow_dir(&entries, &workflow.name).unwrap_or_default();
    let output = build_advance_output(&workflow, new_stage, &flow_dir, &wf_dir);
    print_output(&output);
    Ok(())
}

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
        process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// `awf flow history [--flow=<path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow history [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn history(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    let flow = load_flow_or_exit(&flow_dir)?;

    let entries: Vec<FlowHistoryEntry> = flow
        .stage_history
        .iter()
        .map(|entry| {
            let status = match entry.status {
                crate::model::flow::StageStatus::Completed => "completed",
                crate::model::flow::StageStatus::Failed => "failed",
                crate::model::flow::StageStatus::ForceCompleted => {
                    "force_completed"
                }
                crate::model::flow::StageStatus::Skipped => "skipped",
            };
            FlowHistoryEntry {
                stage: entry.stage.clone(),
                time: entry.time.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                status: status.to_owned(),
                force_reason: entry.force_reason.clone(),
            }
        })
        .collect();

    let output = FlowHistoryOutput {
        flow_path: flow_dir.display().to_string(),
        flow_started_at: flow
            .created_at
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
        entries,
    };
    print_output(&output);
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow archive [--flow=<path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow archive [--flow=<path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn archive(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    match fs::archive_flow(&flow_dir) {
        Ok(()) => {
            let output = FlowArchiveOutput {
                archived_flow: flow_dir.display().to_string(),
            };
            print_output(&output);
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

/// Builds the `FlowAdvanceOutput` for the newly entered stage.
fn build_advance_output(
    workflow: &Workflow,
    new_stage: String,
    flow_dir: &Path,
    wf_dir: &Path,
) -> FlowAdvanceOutput {
    let is_final = workflow.stages.last().is_some_and(|s| s.name == new_stage);
    let next_action = workflow
        .stages
        .iter()
        .find(|s| s.name == new_stage)
        .map_or(FirstStageAction::RunCommand, |s| match &s.kind {
            StageKind::Agent { prompt, prompt_file } => {
                FirstStageAction::AgentComplete {
                    prompt: build_agent_prompt_from_fields(
                        prompt.as_deref(),
                        prompt_file.as_deref(),
                        wf_dir,
                    ),
                }
            }
            StageKind::Command { .. } => FirstStageAction::RunCommand,
        });
    FlowAdvanceOutput {
        stage: new_stage,
        is_final,
        flow_path: flow_dir.display().to_string(),
        next_action,
    }
}

/// Builds an [`AgentPrompt`] from the raw `prompt`/`prompt_file` field values
/// and the workflow directory.
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
    let stage = current_stage_or_exit(workflow, &flow);
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
    let stage = current_stage_or_exit(workflow, flow);
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
