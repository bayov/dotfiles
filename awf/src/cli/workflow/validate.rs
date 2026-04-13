//! Handler for `awf workflow validate`.

use std::env;
use std::fmt;
use std::process;

use owo_colors::OwoColorize as _;

use crate::cli::output::print_output;
use crate::config::WorkflowSearchEntry;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Output payload for `awf workflow validate` (success path).
#[derive(Debug)]
pub struct WorkflowValidateOutput {
    pub name: String,
}

impl fmt::Display for WorkflowValidateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Workflow {}: {}", self.name.blue(), "✓ OK".green())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf workflow validate [name]`.
///
/// When `name` is given, validates that workflow.  When omitted, discovers and
/// validates every workflow across all scopes.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if a workflow cannot be
/// loaded/parsed.
pub fn validate(name: Option<&str>) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let names: Vec<String> = if let Some(n) = name {
        vec![n.to_string()]
    } else {
        let listings = fs::list_workflows(&entries)?;
        if listings.is_empty() {
            anstream::eprintln!(
                "No workflows found. Create one with: awf workflow new \
                 <name>"
            );
            return Ok(());
        }
        listings
            .into_iter()
            .map(|l| format!("{}:{}", l.scope, l.name))
            .collect()
    };

    let mut any_failed = false;
    for n in &names {
        if validate_single(n, &entries, &config.commands)? {
            any_failed = true;
        }
    }
    if any_failed {
        process::exit(1);
    }
    Ok(())
}

/// Validates a single workflow by name.  Returns `true` if validation failed,
/// `false` if it passed.
fn validate_single(
    name: &str,
    entries: &[WorkflowSearchEntry],
    config_commands: &std::collections::HashMap<String, String>,
) -> color_eyre::Result<bool> {
    let workflow = crate::cli::load_workflow_or_exit(entries, name)?;
    let errors = crate::cli::collect_workflow_errors(
        &workflow,
        entries,
        config_commands,
    );

    if errors.is_empty() {
        let output = WorkflowValidateOutput { name: name.to_string() };
        print_output(&output);
        Ok(false)
    } else {
        anstream::eprintln!("Workflow {}: {}", name.blue(), "✗ FAILED".red());
        for error in &errors {
            anstream::eprintln!("  {} {error}", "·".red());
        }
        Ok(true)
    }
}
