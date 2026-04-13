//! Handler for `awf flow validate`.

use std::collections::HashSet;
use std::env;
use std::fmt;

use owo_colors::OwoColorize as _;

use crate::cli::output::eprint_error;
use crate::cli::output::eprint_warning;
use crate::cli::output::print_output;

// ---------------------------------------------------------------------------
// Output type
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
// Handler
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
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let flow_dir = super::resolve_or_exit(flow_flag, &config)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let flow = super::load_flow_or_exit(&flow_dir)?;
    let workflow = crate::cli::load_workflow_or_exit(&entries, &flow.workflow)?;

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

    let actual = super::collect_artifacts(&flow_dir);
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
