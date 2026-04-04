//! Handlers for `awf flow` subcommands: `new`, `list`, and `status`.
//!
//! Each public function corresponds to one CLI subcommand and follows the same
//! pattern: locate the `.awf/` root, call into the store layer, and print
//! human-readable output. Errors that the user should act on (e.g. a flow
//! already exists, or no flow ID can be resolved) are printed to stderr and
//! cause a non-zero exit; unexpected I/O or parse errors are propagated up to
//! `main` for rich error reporting via `color_eyre`.

use std::env;
use std::fmt;
use std::process;

use serde::Serialize;

use crate::cli::output::OutputFormat;
use crate::model::flow::StepStatus;
use crate::result::Error;
use crate::store::fs;
use crate::store::resolve::find_awf_root;
use crate::store::resolve::resolve_flow_id;
use crate::store::resolve::resolve_state_dir;

// ---------------------------------------------------------------------------
// C6 — `awf flow new <name> --workflow <workflow>`
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
    let awf_root = find_awf_root(&env::current_dir()?)?.join(".awf");

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

    let state_dir = resolve_state_dir(&awf_root)?;

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
// C7 — `awf flow list [--all]`
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
    let awf_root = find_awf_root(&env::current_dir()?)?.join(".awf");
    let state_dir = resolve_state_dir(&awf_root)?;

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
// C8 — `awf flow status`
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

    let awf_root = find_awf_root(&env::current_dir()?)?.join(".awf");
    let state_dir = resolve_state_dir(&awf_root)?;

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
