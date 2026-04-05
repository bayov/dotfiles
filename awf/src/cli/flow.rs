//! Handlers for `awf flow` subcommands: `new`, `list`, `status`, `next`,
//! `run`, and `step done`.
//!
//! Each public function corresponds to one CLI subcommand and follows the same
//! pattern: locate the `.awf/` root, call into the store layer, and print
//! human-readable output. Errors that the user should act on (e.g. a flow
//! already exists, or no flow ID can be resolved) are printed to stderr and
//! cause a non-zero exit; unexpected I/O or parse errors are propagated up to
//! `main` for rich error reporting via `color_eyre`.

use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::Path;
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
use crate::store::resolve::find_awf_root;
use crate::store::resolve::resolve_flow_id;
use crate::store::resolve::resolve_state_dir;

// ---------------------------------------------------------------------------
// `awf flow new <name> --workflow <workflow>`
// ---------------------------------------------------------------------------

/// Handles `awf flow new <name> --workflow <workflow>`.
///
/// Locates the nearest `.awf/` root, loads the named workflow to determine its
/// first stage, creates a new flow instance in the state directory, and prints
/// a confirmation message.
///
/// If a flow with the given name already exists, a friendly error message is
/// printed to stderr and the process exits with code 1. All other errors are
/// propagated to the caller.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the named workflow does not exist or cannot be parsed.
pub fn new(name: &str, workflow: &str) -> color_eyre::Result<()> {
    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");

    // Load the workflow to obtain the name of its first stage.
    let wf = fs::load_workflow(&awf_root, workflow)?;

    // A workflow with no stages is technically valid at the parse level, but
    // a flow cannot be created without a starting stage.
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

    let state_dir = resolve_state_dir(&project_root)?;

    match fs::create_flow(&state_dir, name, workflow, first_stage) {
        Ok(()) => {
            println!(
                "Created flow {name} (workflow: {workflow}, stage: \
                 {first_stage})"
            );
            Ok(())
        }
        Err(Error::FlowAlreadyExists { id }) => {
            eprintln!("error: flow {id:?} already exists");
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
/// Locates the nearest `.awf/` root, scans the state directory for flows, and
/// prints a tabular summary. Active flows are always shown; archived flows are
/// included only when `include_archived` is `true`.
///
/// When `--all` is given, an extra `ARCHIVED` column is added showing `yes` or
/// `no` for each row. When the list is empty, a hint on how to create the first
/// flow is printed instead.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no `.awf/` root can be
/// found.
pub fn list(include_archived: bool) -> color_eyre::Result<()> {
    let project_root = find_awf_root(&env::current_dir()?)?;
    let state_dir = resolve_state_dir(&project_root)?;

    let flows = fs::list_flows(&state_dir, include_archived)?;

    if flows.is_empty() {
        println!(
            "No flows found. Create one with: awf flow new <name> \
             --workflow <workflow>"
        );
        return Ok(());
    }

    // Calculate column widths from the data so the table stays aligned
    // regardless of content length.
    let id_width =
        flows.iter().map(|f| f.id.len()).max().unwrap_or(0).max("ID".len());

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

    // Header row.
    if include_archived {
        println!(
            "{:<id_width$}  {:<wf_width$}  {:<stage_width$}  ARCHIVED",
            "ID", "WORKFLOW", "STAGE"
        );
        println!(
            "{:-<id_width$}  {:-<wf_width$}  {:-<stage_width$}  --------",
            "", "", ""
        );
    } else {
        println!(
            "{:<id_width$}  {:<wf_width$}  {:<stage_width$}",
            "ID", "WORKFLOW", "STAGE"
        );
        println!(
            "{:-<id_width$}  {:-<wf_width$}  {:-<stage_width$}",
            "", "", ""
        );
    }

    // Data rows.
    for f in &flows {
        if include_archived {
            let archived_label = if f.archived { "yes" } else { "no" };
            println!(
                "{:<id_width$}  {:<wf_width$}  {:<stage_width$}  {archived_label}",
                f.id, f.workflow, f.current_stage
            );
        } else {
            println!(
                "{:<id_width$}  {:<wf_width$}  {:<stage_width$}",
                f.id, f.workflow, f.current_stage
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow status`
// ---------------------------------------------------------------------------

/// A single step's status as computed for the `status` command output.
#[derive(Debug, Serialize)]
pub struct StepStatusLine {
    /// The name of the step as defined in the workflow.
    pub name: String,

    /// The computed status: `"pending"`, `"pass"`, or `"fail"`.
    pub status: String,
}

/// The complete status payload for `awf flow status`.
///
/// Implements [`fmt::Display`] for the human-readable format and
/// [`serde::Serialize`] for JSON output.
#[derive(Debug, Serialize)]
pub struct FlowStatusOutput {
    /// The unique identifier of the flow.
    pub flow: String,

    /// The name of the current stage.
    pub stage: String,

    /// The status of each step in the current stage.
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

/// Handles `awf flow status [--flow <id>]`.
///
/// Resolves the active flow ID from the `--flow` flag or the `AWF_ACTIVE_FLOW`
/// environment variable, loads the flow and its associated workflow, and prints
/// a summary of each step in the current stage along with its completion
/// status.
///
/// Step statuses are derived from the step log filtered to entries for the
/// current stage: if no entry exists the step is `pending`; otherwise the last
/// recorded entry determines whether it is `pass` or `fail`.
///
/// When `fmt` is [`OutputFormat::Json`], the output is serialized as JSON
/// rather than printed in the human-readable format.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the workflow or flow state cannot be loaded.
pub fn status(
    flow_flag: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let flow_id = match resolve_flow_id(flow_flag) {
        Ok(id) => id,
        Err(Error::FlowIdRequired) => {
            eprintln!(
                "error: no flow specified; pass --flow <id> or set \
                 AWF_ACTIVE_FLOW"
            );
            process::exit(1);
        }
        Err(e) => return Err(e.into()),
    };

    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");
    let state_dir = resolve_state_dir(&project_root)?;

    let flow = fs::load_flow(&state_dir, &flow_id)?;
    let workflow = fs::load_workflow(&awf_root, &flow.workflow)?;

    // Locate the current stage definition in the workflow.
    let current_stage =
        workflow.stages.iter().find(|s| s.name == flow.current_stage);

    let steps = match current_stage {
        Some(stage) => {
            // Collect all step-log entries that belong to the current stage
            // visit so we can determine the last recorded status per step.
            let current_stage_entries: Vec<_> = flow
                .step_log
                .iter()
                .filter(|e| e.stage == flow.current_stage)
                .collect();

            stage
                .steps
                .iter()
                .map(|step| {
                    // Find the last log entry for this step in the current
                    // stage (entries are ordered oldest-first, so we scan in
                    // reverse).
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
        // If the current stage is not found in the workflow (e.g. the
        // workflow was edited after the flow was created), report no steps.
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
// `awf flow next [--flow <id>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow next [--flow <id>]`.
///
/// Resolves the active flow ID, loads the flow and its associated workflow,
/// collects the set of already-produced artifact names from the filesystem,
/// then calls [`next_action`] and prints a human-readable description of the
/// highest-priority action the user should take next.
///
/// Output per action variant:
///
/// - [`NextAction::RunStep`]: print the step name and the command to run it.
/// - [`NextAction::CompleteAgentStep`]: print the prompt file contents (if the
///   step has one) and the command to mark it complete.
/// - [`NextAction::ProduceArtifact`]: print the artifact name and the command
///   to register it.
/// - [`NextAction::CanAdvance`]: print the next stage name and the `advance`
///   command.
/// - [`NextAction::Done`]: print a completion message.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the workflow or flow state cannot be loaded.
pub fn next(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let flow_id = resolve_or_exit(flow_flag)?;

    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");
    let state_dir = resolve_state_dir(&project_root)?;

    let flow = fs::load_flow(&state_dir, &flow_id)?;
    let workflow = fs::load_workflow(&awf_root, &flow.workflow)?;

    let artifacts = collect_artifacts(&state_dir, &flow_id);

    match next_action(&workflow, &flow, &artifacts) {
        NextAction::RunStep(step) => {
            println!(
                "[command] {} — next step to run: awf flow run {}",
                step.name, step.name
            );
        }
        NextAction::CompleteAgentStep(step) => {
            // Load and print the prompt file when the step declares one.
            if let StepKind::Agent { prompt: Some(ref filename), .. } =
                step.kind
            {
                match fs::load_prompt_file(&awf_root, &flow.workflow, filename)
                {
                    Ok(contents) => println!("{contents}"),
                    Err(e) => {
                        eprintln!(
                            "warning: could not load prompt file {filename:?}: \
                             {e}"
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
            // Find the name of the stage that follows the current one.
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
// `awf flow run <step> [--flow <id>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow run <step> [--flow <id>]`.
///
/// Resolves the active flow, finds the named step in the current stage,
/// resolves the shell command to execute, runs it, records the pass/fail
/// outcome in the flow's step log, saves the updated state, and prints a
/// one-line result summary.
///
/// If the step is an agent step (not a command step) an error is printed to
/// stderr and the process exits with code 1.
///
/// After a failing run, a hint is printed showing how to re-run the step once
/// the error has been fixed.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the workflow or flow state cannot be loaded.
pub fn run(step_name: &str, flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let flow_id = resolve_or_exit(flow_flag)?;

    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");
    let state_dir = resolve_state_dir(&project_root)?;

    let flow = fs::load_flow(&state_dir, &flow_id)?;
    let workflow = fs::load_workflow(&awf_root, &flow.workflow)?;

    // Locate the step by name in the current stage.
    let step = find_step_in_current_stage(&workflow, &flow, step_name)
        .unwrap_or_else(|| {
            eprintln!(
                "error: step {step_name:?} not found in current stage {:?}",
                flow.current_stage
            );
            process::exit(1);
        });

    // Reject agent steps — they must be completed with `awf flow step done`.
    if matches!(step.kind, StepKind::Agent { .. }) {
        eprintln!(
            "error: Step {step_name:?} is an agent step. Use \
             `awf flow step done {step_name}` instead."
        );
        process::exit(1);
    }

    // Resolve the shell command string from the step definition.
    let shell_cmd =
        resolve_shell_command(step, &workflow).unwrap_or_else(|| {
            eprintln!(
                "error: step {step_name:?} has no resolvable shell command"
            );
            process::exit(1);
        });

    // Run the command and record timing information.
    let started_at = Utc::now();
    let result = run_command(shell_cmd)?;

    // Record the outcome in the flow's step log.
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

    fs::save_flow(&state_dir, &flow)?;

    // Print the one-line result summary.
    if result.exit_code == 0 {
        println!("PASS  {}  ({}ms)", shell_cmd, result.duration_ms);
    } else {
        println!("FAIL  {}  ({}ms)", shell_cmd, result.duration_ms);
        println!("Fix the error, then run: awf flow run {step_name}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow step done <step> [--flow <id>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow step done <step> [--flow <id>]`.
///
/// Resolves the active flow, finds the named step in the current stage,
/// records a passing [`StepLogEntry`] with no timing data (appropriate for
/// agent steps whose duration is not tracked), saves the updated state, and
/// prints a confirmation message.
///
/// If the step is a command step (not an agent step) an error is printed to
/// stderr and the process exits with code 1.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the workflow or flow state cannot be loaded.
///
/// [`StepLogEntry`]: crate::model::flow::StepLogEntry
pub fn step_done(
    step_name: &str,
    flow_flag: Option<&str>,
) -> color_eyre::Result<()> {
    let flow_id = resolve_or_exit(flow_flag)?;

    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");
    let state_dir = resolve_state_dir(&project_root)?;

    let flow = fs::load_flow(&state_dir, &flow_id)?;
    let workflow = fs::load_workflow(&awf_root, &flow.workflow)?;

    // Locate the step by name in the current stage.
    let step = find_step_in_current_stage(&workflow, &flow, step_name)
        .unwrap_or_else(|| {
            eprintln!(
                "error: step {step_name:?} not found in current stage {:?}",
                flow.current_stage
            );
            process::exit(1);
        });

    // Reject command steps — they must be completed with `awf flow run`.
    if matches!(step.kind, StepKind::Command { .. }) {
        eprintln!(
            "error: Step {step_name:?} is a command step. Use \
             `awf flow run {step_name}` instead."
        );
        process::exit(1);
    }

    let stage = flow.current_stage.clone();
    let flow = record_step_pass(flow, step, &stage, None, None, None);

    fs::save_flow(&state_dir, &flow)?;

    println!("Step {step_name} marked complete.");

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow artifact put <name> <file> [--flow <id>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow artifact put <name> <file> [--flow <id>]`.
///
/// Resolves the active flow ID, locates the `.awf/` root and state directory,
/// copies the given file into the flow's `artifacts/` directory under the
/// provided name, and prints a confirmation message.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the source file cannot be copied.
pub fn artifact_put(
    name: &str,
    file: &std::path::Path,
    flow_flag: Option<&str>,
) -> color_eyre::Result<()> {
    let flow_id = resolve_or_exit(flow_flag)?;

    let project_root = find_awf_root(&env::current_dir()?)?;
    let state_dir = resolve_state_dir(&project_root)?;

    fs::put_artifact(&state_dir, &flow_id, name, file)?;

    println!("Artifact {name} registered.");

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow advance [--force] [--flow <id>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow advance [--force] [--flow <id>]`.
///
/// Resolves the active flow, loads the workflow and flow state, then:
///
/// 1. Unless `--force` is given, calls [`can_advance`] to ensure all steps have
///    passed and all required artifacts are present. Prints a descriptive error
///    and exits with code 1 on any blocker.
/// 2. Calls [`can_enter_stage`] for the next stage to check entry conditions:
///    - [`EnterResult::MissingRequires`]: prints a friendly error and exits
///      with code 1 (not bypassed by `--force`).
///    - [`EnterResult::AlreadyCompleted`]: prints a warning and proceeds (v1
///      does not support interactive confirmation prompts).
/// 3. Calls [`advance_stage`] to produce the updated flow state.
/// 4. Saves the updated flow and prints `Entered stage: <name>`.
///
/// If the flow is already on its last stage, `advance_stage` returns
/// [`Error::AlreadyOnLastStage`]; this is propagated as an error to the
/// caller.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the workflow or flow state cannot be loaded.
pub fn advance(flow_flag: Option<&str>, force: bool) -> color_eyre::Result<()> {
    let flow_id = resolve_or_exit(flow_flag)?;

    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");
    let state_dir = resolve_state_dir(&project_root)?;

    let flow = fs::load_flow(&state_dir, &flow_id)?;
    let workflow = fs::load_workflow(&awf_root, &flow.workflow)?;

    let artifacts = collect_artifacts(&state_dir, &flow_id);

    // Step 1: check that the current stage is complete (unless --force).
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

    // Step 2: determine the next stage and check entry conditions.
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

        // Ensure there is a next stage to enter.
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

        // Missing required inputs: block even with --force since the next
        // stage cannot execute without them.
        EnterResult::MissingRequires(missing) => {
            eprintln!(
                "ERROR: Cannot enter stage `{next_stage_name}`. Missing \
                 required artifacts: {}",
                missing.join(", ")
            );
            process::exit(1);
        }

        // Already completed: warn and proceed (v1 omits interactive prompt).
        EnterResult::AlreadyCompleted { produces } => {
            eprintln!(
                "WARNING: Stage `{next_stage_name}` has already been \
                 completed (produces: {}). Re-entering it will overwrite \
                 previous results.",
                produces.join(", ")
            );
        }
    }

    // Step 3: perform the stage transition.
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

    // Step 4: persist and report.
    let new_stage = updated_flow.current_stage.clone();
    fs::save_flow(&state_dir, &updated_flow)?;

    println!("Entered stage: {new_stage}");

    // If the new stage is the last one in the workflow, hint that we are done.
    let is_last = workflow.stages.last().is_some_and(|s| s.name == new_stage);
    if is_last {
        println!("This is the final stage.");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// `awf flow history [--flow <id>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow history [--flow <id>]`.
///
/// Resolves the active flow, loads its state, and prints a chronological
/// history of stage transitions and step executions grouped by stage.
///
/// Output format (human-readable):
///
/// ```text
/// [design] entered 2026-04-04T10:00:00Z
///   write-design  pass  2026-04-04T13:00:00Z
/// [design] exited  2026-04-04T14:00:00Z
/// [implement] entered ...
///   format  pass  320ms
///   lint    fail  ...
///   lint    pass  ...
/// ```
///
/// Completed stages are drawn from `stage_history`; the current (in-progress)
/// stage is appended at the end using `current_stage` and `stage_entered_at`.
/// Step log entries are filtered per stage by the `stage` field on each entry.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures, if no `.awf/` root can be
/// found, or if the flow state cannot be loaded.
pub fn history(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let flow_id = resolve_or_exit(flow_flag)?;

    let project_root = find_awf_root(&env::current_dir()?)?;
    let state_dir = resolve_state_dir(&project_root)?;

    let flow = fs::load_flow(&state_dir, &flow_id)?;

    // Iterate completed stages from history.
    for entry in &flow.stage_history {
        println!(
            "[{}] entered {}",
            entry.stage,
            entry.entered_at.format("%Y-%m-%dT%H:%M:%SZ")
        );

        // Print all step log entries that belong to this stage.
        print_step_log_for_stage(&flow, &entry.stage);

        if let Some(exited_at) = entry.exited_at {
            println!(
                "[{}] exited  {}",
                entry.stage,
                exited_at.format("%Y-%m-%dT%H:%M:%SZ")
            );
        }
    }

    // Print the current (in-progress) stage, unless it was already in history
    // (which would happen only for repeatable stages re-entered after
    // completion — in practice the current stage is always the active one).
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
// `awf flow archive [--flow <id>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow archive [--flow <id>]`.
///
/// Resolves the active flow ID, locates the state directory, and calls
/// [`fs::archive_flow`] to move the flow from `flows/active/` to
/// `flows/archived/` and mark it as archived.
///
/// On success prints:
/// `Archived flow <flow_id> → <state_dir>/flows/archived/<flow_id>/`
///
/// On [`Error::FlowNotFound`] a friendly message is printed and the process
/// exits with code 1.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no `.awf/` root can be
/// found.
pub fn archive(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let flow_id = resolve_or_exit(flow_flag)?;

    let project_root = find_awf_root(&env::current_dir()?)?;
    let state_dir = resolve_state_dir(&project_root)?;

    match fs::archive_flow(&state_dir, &flow_id) {
        Ok(()) => {
            let archived_path = state_dir.join("flows/archived").join(&flow_id);
            println!("Archived flow {flow_id} → {}", archived_path.display());
            Ok(())
        }
        Err(Error::FlowNotFound { id }) => {
            eprintln!("error: flow {id:?} not found");
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Prints all step log entries for `stage_name` from the flow's step log.
///
/// Each entry is indented two spaces and rendered as:
/// `  <step>  <pass|fail>  <timestamp or duration>`
///
/// For command steps (`step_type == "command"`), the duration in milliseconds
/// is shown when available; otherwise the `started_at` timestamp is used.
/// For agent steps (`step_type == "agent"`), `completed_at` is shown.
fn print_step_log_for_stage(flow: &FlowState, stage_name: &str) {
    for entry in flow.step_log.iter().filter(|e| e.stage == stage_name) {
        let status_str = match entry.status {
            StepStatus::Pass => "pass",
            StepStatus::Fail => "fail",
        };

        // Build a timing string that is as informative as possible.
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

/// Resolves the flow ID from the flag or environment, printing a friendly
/// error message and exiting with code 1 if no ID can be determined.
fn resolve_or_exit(flow_flag: Option<&str>) -> color_eyre::Result<String> {
    match resolve_flow_id(flow_flag) {
        Ok(id) => Ok(id),
        Err(Error::FlowIdRequired) => {
            eprintln!(
                "error: no flow specified; pass --flow <id> or set \
                 AWF_ACTIVE_FLOW"
            );
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

/// Scans `<state_dir>/flows/active/<flow_id>/artifacts/` and returns the set
/// of filenames present on disk.
///
/// Returns an empty set when the artifacts directory does not exist (e.g. for
/// a freshly-created flow that has not yet produced any artifacts). Any I/O
/// error encountered while reading individual directory entries is silently
/// ignored so that a partially-corrupt artifact directory does not block the
/// `next` command.
fn collect_artifacts(state_dir: &Path, flow_id: &str) -> HashSet<String> {
    let artifact_dir =
        state_dir.join("flows/active").join(flow_id).join("artifacts");

    // Directory absent is the normal case for a new flow.
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

/// Finds a step by `step_name` in the stage named by `flow.current_stage`.
///
/// Returns `None` when the current stage is not found in the workflow or when
/// no step with the given name exists in that stage.
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
/// If the step uses the `command` key, the corresponding value is looked up in
/// `workflow.commands`. If the step uses the inline `run` key, that string is
/// returned directly. Returns `None` when neither key resolves to a command
/// (which should not occur in a validated workflow).
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
