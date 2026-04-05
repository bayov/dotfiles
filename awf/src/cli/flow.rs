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
use crate::cli::output::render;
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
            "Created flow {} (workflow: {}, stage: {})",
            self.path, self.workflow, self.stage
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

        let path_width = self
            .flows
            .iter()
            .map(|e| e.path.len())
            .max()
            .unwrap_or(0)
            .max("PATH".len());
        let wf_width = self
            .flows
            .iter()
            .map(|e| e.workflow.len())
            .max()
            .unwrap_or(0)
            .max("WORKFLOW".len());
        let stage_width = self
            .flows
            .iter()
            .map(|e| e.stage.len())
            .max()
            .unwrap_or(0)
            .max("STAGE".len());

        if self.include_archived {
            writeln!(
                f,
                "{:<path_width$}  {:<wf_width$}  {:<stage_width$}  ARCHIVED",
                "PATH", "WORKFLOW", "STAGE"
            )?;
            writeln!(
                f,
                "{:-<path_width$}  {:-<wf_width$}  {:-<stage_width$}  --------",
                "", "", ""
            )?;
            for entry in &self.flows {
                let archived_label = if entry.archived { "yes" } else { "no" };
                writeln!(
                    f,
                    "{:<path_width$}  {:<wf_width$}  {:<stage_width$}  \
                     {archived_label}",
                    entry.path, entry.workflow, entry.stage
                )?;
            }
        } else {
            writeln!(
                f,
                "{:<path_width$}  {:<wf_width$}  {:<stage_width$}",
                "PATH", "WORKFLOW", "STAGE"
            )?;
            writeln!(
                f,
                "{:-<path_width$}  {:-<wf_width$}  {:-<stage_width$}",
                "", "", ""
            )?;
            for entry in &self.flows {
                writeln!(
                    f,
                    "{:<path_width$}  {:<wf_width$}  {:<stage_width$}",
                    entry.path, entry.workflow, entry.stage
                )?;
            }
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
        writeln!(f, "Flow:    {}", self.flow)?;
        writeln!(f, "Stage:   {}", self.stage)?;
        writeln!(f, "Steps:")?;
        for step in &self.steps {
            writeln!(f, "  {}  [{}]", step.name, step.status)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow next`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow next`.
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
        writeln!(f, "{}", self.message)
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
        let status_upper = self.status.to_uppercase();
        writeln!(
            f,
            "{status_upper}  {}  ({}ms)",
            self.command, self.duration_ms
        )?;
        if self.status == "fail" {
            writeln!(f, "Fix the error, then run: awf flow run {}", self.step)?;
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
        writeln!(f, "Step {} marked complete.", self.step)
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
        writeln!(f, "Flow artifacts valid.")
    }
}

// ---------------------------------------------------------------------------
// Output types — `awf flow advance`
// ---------------------------------------------------------------------------

/// Output payload for `awf flow advance`.
#[derive(Debug, Serialize)]
pub struct FlowAdvanceOutput {
    pub stage: String,
    pub is_final: bool,
}

impl fmt::Display for FlowAdvanceOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Entered stage: {}", self.stage)?;
        if self.is_final {
            writeln!(f, "This is the final stage.")?;
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
    pub timing: Option<String>,
}

/// A stage record within the history output.
#[derive(Debug, Serialize)]
pub struct FlowStageRecord {
    pub stage: String,
    pub entered_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exited_at: Option<String>,
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
            writeln!(f, "[{}] entered {}", entry.stage, entry.entered_at)?;
            for step in &entry.steps {
                if let Some(ref timing) = step.timing {
                    writeln!(f, "  {}  {}  {timing}", step.step, step.status)?;
                } else {
                    writeln!(f, "  {}  {}", step.step, step.status)?;
                }
            }
            if let Some(ref exited_at) = entry.exited_at {
                writeln!(f, "[{}] exited  {}", entry.stage, exited_at)?;
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
    pub path: String,
}

impl fmt::Display for FlowArchiveOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Archived flow: {}", self.path)
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
    flow_flag: Option<&str>,
    workflow: &str,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
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
            let output = FlowNewOutput {
                path: flow_dir.display().to_string(),
                workflow: workflow.to_string(),
                stage: first_stage.to_string(),
            };
            print!("{}", render(&output, fmt));
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
pub fn list(
    include_archived: bool,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
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
    print!("{}", render(&output, fmt));
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

    print!("{}", render(&output, fmt));
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
pub fn next(
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
                            eprintln!(
                                "warning: could not load prompt file \
                                 {filename:?}: {e}"
                            );
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
                    "All steps complete. Run `awf flow advance` to move to \
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

    print!("{}", render(&output, fmt));
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

    let status = if result.exit_code == 0 { "pass" } else { "fail" };
    let output = FlowRunOutput {
        step: step_name.to_string(),
        command: shell_cmd.to_string(),
        status: status.to_string(),
        duration_ms: result.duration_ms,
    };
    print!("{}", render(&output, fmt));
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

    let output = FlowStepDoneOutput { step: step_name.to_string() };
    print!("{}", render(&output, fmt));
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
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = load_flow_or_exit(&flow_dir)?;
    let workflow = fs::load_workflow(&entries, &flow.workflow)?;

    let completed_stage_names: HashSet<&str> = flow
        .stage_history
        .iter()
        .filter(|e| e.exited_at.is_some())
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
        print!("{}", render(&output, fmt));
        return Ok(());
    }

    for name in &missing {
        eprintln!("missing artifact: {name}");
    }
    for name in &redundant {
        eprintln!("redundant artifact: {name}");
    }

    color_eyre::eyre::bail!("flow artifact validation failed");
}

// ---------------------------------------------------------------------------
// `awf flow advance [--force] [--flow <path>]`
// ---------------------------------------------------------------------------

/// Handles `awf flow advance [--force] [--flow <path>]`.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn advance(
    flow_flag: Option<&str>,
    force: bool,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
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

    let is_final = workflow.stages.last().is_some_and(|s| s.name == new_stage);
    let output = FlowAdvanceOutput { stage: new_stage, is_final };
    print!("{}", render(&output, fmt));
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
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    let flow = load_flow_or_exit(&flow_dir)?;

    let mut entries: Vec<FlowStageRecord> = flow
        .stage_history
        .iter()
        .map(|entry| FlowStageRecord {
            stage: entry.stage.clone(),
            entered_at: entry
                .entered_at
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            exited_at: entry
                .exited_at
                .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            steps: build_step_records(&flow, &entry.stage),
        })
        .collect();

    let current_already_shown =
        flow.stage_history.iter().any(|e| e.stage == flow.current_stage);

    if !current_already_shown {
        entries.push(FlowStageRecord {
            stage: flow.current_stage.clone(),
            entered_at: flow
                .stage_entered_at
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            exited_at: None,
            steps: build_step_records(&flow, &flow.current_stage),
        });
    }

    let output = FlowHistoryOutput { entries };
    print!("{}", render(&output, fmt));
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
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = resolve_or_exit(flow_flag, &config)?;

    match fs::archive_flow(&flow_dir) {
        Ok(()) => {
            let output =
                FlowArchiveOutput { path: flow_dir.display().to_string() };
            print!("{}", render(&output, fmt));
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
            let timing = if entry.step_type == "command" {
                entry.duration_ms.map(|ms| format!("{ms}ms")).or_else(|| {
                    entry
                        .started_at
                        .map(|ts| ts.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                })
            } else {
                entry
                    .completed_at
                    .map(|ts| ts.format("%Y-%m-%dT%H:%M:%SZ").to_string())
            };
            FlowStepRecord {
                step: entry.step.clone(),
                status: status_str.to_string(),
                step_type: entry.step_type.clone(),
                timing,
            }
        })
        .collect()
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
