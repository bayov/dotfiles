//! Handlers for `awf flow` subcommands.

use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::process;

use chrono::Utc;
use serde::Serialize;

use crate::cli::output::OutputFormat;
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
use crate::store::resolve::find_project_root;
use crate::store::resolve::resolve_flow_path;

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
pub fn new(flow_flag: Option<&str>, workflow: &str) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let wf = fs::load_workflow(&entries, workflow)?;

    let first_stage = wf.stages.first().map_or_else(
        || {
            eprintln!(
                "error: workflow {workflow:?} has no stages; \
                 cannot create a flow"
            );
            process::exit(1);
        },
        |s| s.name.as_str(),
    );

    match fs::create_flow(&flow_dir, workflow, first_stage) {
        Ok(()) => {
            println!(
                "Created flow {} (workflow: {workflow}, stage: {first_stage})",
                flow_dir.display()
            );
            Ok(())
        }
        Err(Error::FlowAlreadyExists { .. }) => {
            eprintln!("error: flow already exists at {}", flow_dir.display());
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
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
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;

    let all_flows = fs::scan_flows(&config.flow_search_dirs)?;
    let flows: Vec<_> = if include_archived {
        all_flows
    } else {
        all_flows.into_iter().filter(|f| !f.archived).collect()
    };

    if flows.is_empty() {
        if include_archived {
            println!("No flows found.");
        } else {
            println!(
                "No flows found. Create one with: awf flow new --flow <path> \
                 --workflow <workflow>"
            );
        }
        return Ok(());
    }

    let path_width = flows
        .iter()
        .map(|f| f.path.display().to_string().len())
        .max()
        .unwrap_or(0)
        .max("PATH".len());

    let wf_width = flows
        .iter()
        .map(|f| f.workflow.len())
        .max()
        .unwrap_or(0)
        .max("WORKFLOW".len());

    let stage_width = flows
        .iter()
        .map(|f| f.current_stage.len())
        .max()
        .unwrap_or(0)
        .max("STAGE".len());

    if include_archived {
        println!(
            "{:<path_width$}  {:<wf_width$}  {:<stage_width$}  ARCHIVED",
            "PATH", "WORKFLOW", "STAGE"
        );
        println!(
            "{:-<path_width$}  {:-<wf_width$}  {:-<stage_width$}  --------",
            "", "", ""
        );
        for f in &flows {
            let archived_label = if f.archived { "yes" } else { "no" };
            let path_str = f.path.display().to_string();
            println!(
                "{:<path_width$}  {:<wf_width$}  {:<stage_width$}  {archived_label}",
                path_str, f.workflow, f.current_stage
            );
        }
    } else {
        println!(
            "{:<path_width$}  {:<wf_width$}  {:<stage_width$}",
            "PATH", "WORKFLOW", "STAGE"
        );
        println!(
            "{:-<path_width$}  {:-<wf_width$}  {:-<stage_width$}",
            "", "", ""
        );
        for f in &flows {
            let path_str = f.path.display().to_string();
            println!(
                "{:<path_width$}  {:<wf_width$}  {:<stage_width$}",
                path_str, f.workflow, f.current_stage
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow status [--flow <path>]`
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
        writeln!(f, "Flow:    {}", self.flow)?;
        writeln!(f, "Stage:   {}", self.stage)?;
        writeln!(f, "Steps:")?;
        for step in &self.steps {
            writeln!(f, "  {}  [{}]", step.name, step.status)?;
        }
        Ok(())
    }
}

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
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = fs::load_workflow(&entries, &flow.workflow)?;

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

    print!("{}", crate::cli::output::render(&output, fmt));
    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow next [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow next [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn next(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = fs::load_workflow(&entries, &flow.workflow)?;
    let artifacts = collect_artifacts(&flow_dir);

    match next_action(&workflow, &flow, &artifacts) {
        NextAction::RunStep(step) => {
            println!(
                "[command] {} — next step to run: awf flow run {}",
                step.name, step.name
            );
        }
        NextAction::CompleteAgentStep(step) => {
            if let StepKind::Agent { prompt: Some(ref filename), .. } =
                step.kind
            {
                match fs::load_prompt_file(&entries, &flow.workflow, filename) {
                    Ok(contents) => println!("{contents}"),
                    Err(e) => {
                        eprintln!(
                            "warning: could not load prompt file {filename:?}: {e}"
                        );
                    }
                }
            }
            println!("Mark complete with: awf flow step done {}", step.name);
        }
        NextAction::ProduceArtifact(name) => {
            println!("All steps complete. Produce artifact: {name}");
            println!("Register it with: awf flow artifact put {name} <file>");
        }
        NextAction::CanAdvance => {
            let next_stage_name = workflow
                .stages
                .iter()
                .position(|s| s.name == flow.current_stage)
                .and_then(|idx| workflow.stages.get(idx + 1))
                .map_or_else(|| "next".to_string(), |s| s.name.clone());
            println!(
                "All steps complete. Run `awf flow advance` to move to \
                 {next_stage_name}."
            );
        }
        NextAction::Done => {
            println!("All stages complete. The workflow is done.");
        }
    }

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
pub fn run(step_name: &str, flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = fs::load_workflow(&entries, &flow.workflow)?;

    let step = find_step_in_current_stage(&workflow, &flow, step_name)
        .unwrap_or_else(|| {
            eprintln!(
                "error: step {step_name:?} not found in current stage {:?}",
                flow.current_stage
            );
            process::exit(1);
        });

    if matches!(step.kind, StepKind::Agent { .. }) {
        eprintln!(
            "error: Step {step_name:?} is an agent step. Use \
             `awf flow step done {step_name}` instead."
        );
        process::exit(1);
    }

    let shell_cmd =
        resolve_shell_command(step, &workflow).unwrap_or_else(|| {
            eprintln!(
                "error: step {step_name:?} has no resolvable shell command"
            );
            process::exit(1);
        });

    let started_at = Utc::now();
    let result = run_command(shell_cmd)?;

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

    if result.exit_code == 0 {
        println!("PASS  {}  ({}ms)", shell_cmd, result.duration_ms);
    } else {
        println!("FAIL  {}  ({}ms)", shell_cmd, result.duration_ms);
        println!("Fix the error, then run: awf flow run {step_name}");
    }

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
) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = fs::load_workflow(&entries, &flow.workflow)?;

    let step = find_step_in_current_stage(&workflow, &flow, step_name)
        .unwrap_or_else(|| {
            eprintln!(
                "error: step {step_name:?} not found in current stage {:?}",
                flow.current_stage
            );
            process::exit(1);
        });

    if matches!(step.kind, StepKind::Command { .. }) {
        eprintln!(
            "error: Step {step_name:?} is a command step. Use \
             `awf flow run {step_name}` instead."
        );
        process::exit(1);
    }

    let stage = flow.current_stage.clone();
    let flow = record_step_pass(flow, step, &stage, None, None, None);

    fs::save_flow(&flow_dir, &flow)?;

    println!("Step {step_name} marked complete.");

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow artifact put <name> <file> [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow artifact put <name> <file> [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn artifact_put(
    name: &str,
    file: &std::path::Path,
    flow_flag: Option<&str>,
) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    fs::put_artifact(&flow_dir, name, file)?;

    println!("Artifact {name} registered.");

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow advance [--force] [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow advance [--force] [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn advance(flow_flag: Option<&str>, force: bool) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = fs::load_workflow(&entries, &flow.workflow)?;
    let artifacts = collect_artifacts(&flow_dir);

    if !force && let Err(blocker) = can_advance(&workflow, &flow, &artifacts) {
        match blocker {
            AdvanceBlocker::StepFailed(step) => {
                eprintln!(
                    "ERROR: `{step}` failed. Fix the error then run \
                     `awf flow run {step}` before advancing."
                );
            }
            AdvanceBlocker::StepIncomplete(step) => {
                eprintln!(
                    "ERROR: `{step}` is not complete. Complete it before \
                     advancing."
                );
            }
            AdvanceBlocker::MissingArtifact(name) => {
                eprintln!(
                    "ERROR: Missing artifact `{name}`. Register it with \
                     `awf flow artifact put {name} <file>` before \
                     advancing."
                );
            }
        }
        process::exit(1);
    }

    let next_stage_name = {
        let current_idx = workflow
            .stages
            .iter()
            .position(|s| s.name == flow.current_stage)
            .unwrap_or_else(|| {
                eprintln!(
                    "error: current stage {:?} not found in workflow",
                    flow.current_stage
                );
                process::exit(1);
            });

        let Some(next) = workflow.stages.get(current_idx + 1) else {
            eprintln!(
                "error: already on the last stage; nothing to advance to"
            );
            process::exit(1);
        };

        next.name.clone()
    };

    match can_enter_stage(&workflow, &flow, &next_stage_name, &artifacts) {
        EnterResult::Ok => {}
        EnterResult::MissingRequires(missing) => {
            eprintln!(
                "ERROR: Cannot enter stage `{next_stage_name}`. Missing \
                 required artifacts: {}",
                missing.join(", ")
            );
            process::exit(1);
        }
        EnterResult::AlreadyCompleted { produces } => {
            eprintln!(
                "WARNING: Stage `{next_stage_name}` has already been \
                 completed (produces: {}). Re-entering it will overwrite \
                 previous results.",
                produces.join(", ")
            );
        }
    }

    let updated_flow = match advance_stage(&workflow, flow) {
        Ok(f) => f,
        Err(Error::AlreadyOnLastStage) => {
            eprintln!(
                "error: already on the last stage; nothing to advance to"
            );
            process::exit(1);
        }
        Err(e) => return Err(e.into()),
    };

    let new_stage = updated_flow.current_stage.clone();
    fs::save_flow(&flow_dir, &updated_flow)?;

    println!("Entered stage: {new_stage}");

    let is_last = workflow.stages.last().is_some_and(|s| s.name == new_stage);
    if is_last {
        println!("This is the final stage.");
    }

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
pub fn history(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    let flow = load_flow_or_exit(&flow_dir)?;

    for entry in &flow.stage_history {
        println!(
            "[{}] entered {}",
            entry.stage,
            entry.entered_at.format("%Y-%m-%dT%H:%M:%SZ")
        );
        print_step_log_for_stage(&flow, &entry.stage);
        if let Some(exited_at) = entry.exited_at {
            println!(
                "[{}] exited  {}",
                entry.stage,
                exited_at.format("%Y-%m-%dT%H:%M:%SZ")
            );
        }
    }

    let current_already_shown =
        flow.stage_history.iter().any(|e| e.stage == flow.current_stage);

    if !current_already_shown {
        println!(
            "[{}] entered {}",
            flow.current_stage,
            flow.stage_entered_at.format("%Y-%m-%dT%H:%M:%SZ")
        );
        print_step_log_for_stage(&flow, &flow.current_stage);
    }

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
pub fn archive(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    match fs::archive_flow(&flow_dir) {
        Ok(()) => {
            println!("Archived flow: {}", flow_dir.display());
            Ok(())
        }
        Err(Error::FlowNotFound { .. }) => {
            eprintln!("error: flow not found at {}", flow_dir.display());
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Prints all step log entries for `stage_name` from the flow's step log.
fn print_step_log_for_stage(flow: &FlowState, stage_name: &str) {
    for entry in flow.step_log.iter().filter(|e| e.stage == stage_name) {
        let status_str = match entry.status {
            StepStatus::Pass => "pass",
            StepStatus::Fail => "fail",
        };
        let timing = if entry.step_type == "command" {
            entry.duration_ms.map_or_else(
                || {
                    entry.started_at.map_or_else(String::new, |ts| {
                        ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
                    })
                },
                |ms| format!("{ms}ms"),
            )
        } else {
            entry.completed_at.map_or_else(String::new, |ts| {
                ts.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            })
        };
        if timing.is_empty() {
            println!("  {}  {status_str}", entry.step);
        } else {
            println!("  {}  {status_str}  {timing}", entry.step);
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
            eprintln!(
                "error: no flow specified; pass --flow <path> or set \
                 AWF_ACTIVE_FLOW"
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
            eprintln!("error: flow not found at {}", flow_dir.display());
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
fn resolve_shell_command<'a>(
    step: &'a Step,
    workflow: &'a Workflow,
) -> Option<&'a str> {
    match &step.kind {
        StepKind::Command { command: Some(alias), run: None } => {
            workflow.commands.get(alias.as_str()).map(String::as_str)
        }
        StepKind::Command { run: Some(inline), command: None } => {
            Some(inline.as_str())
        }
        _ => None,
    }
}
