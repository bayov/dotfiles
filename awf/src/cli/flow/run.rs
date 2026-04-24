//! Handler for `awf flow run`.

use std::collections::HashMap;
use std::env;
use std::fmt;

use owo_colors::OwoColorize as _;

use crate::cli::output::eprint_error;
use crate::cli::output::print_output;
use crate::engine::command::run_command;
use crate::engine::state::record_stage_fail;
use crate::engine::state::record_stage_pass;
use crate::model::workflow::LinearKind;
use crate::model::workflow::Stage;
use crate::model::workflow::StageKind;
use crate::model::workflow::Workflow;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output type
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
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf flow run [--flow=<path>]`.
///
/// Runs the current stage if it is a command stage.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures.
pub fn run(flow_flag: Option<&str>) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = super::resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = super::load_flow_or_exit(&flow_dir)?;
    let workflow = crate::cli::load_workflow_or_exit(&entries, &flow.workflow)?;

    let stage = super::current_stage_or_exit(&workflow, &flow);

    if !matches!(stage.kind, StageKind::Linear(LinearKind::Command { .. })) {
        eprint_error(format_args!(
            "Stage {:?} is not a command stage. Use `awf flow next-stage` instead.",
            stage.name
        ));
        std::process::exit(1);
    }

    let shell_cmd = resolve_shell_command(stage, &workflow, &config.commands)
        .unwrap_or_else(|| {
            eprint_error(format_args!(
                "stage {:?} has no resolvable shell command",
                stage.name
            ));
            std::process::exit(1);
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
// Private helper
// ---------------------------------------------------------------------------

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
        StageKind::Linear(LinearKind::Command {
            command: Some(alias),
            run: None,
        }) => workflow
            .commands
            .get(alias.as_str())
            .or_else(|| repo_commands.get(alias.as_str()))
            .cloned(),
        StageKind::Linear(LinearKind::Command {
            run: Some(inline),
            command: None,
        }) => Some(inline.clone()),
        _ => None,
    }
}
