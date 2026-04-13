//! Handler for `awf workflow list`.

use std::env;
use std::fmt;

use owo_colors::OwoColorize as _;

use crate::cli::output::print_output;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single workflow entry in the list output.
#[derive(Debug)]
pub struct WorkflowListEntry {
    pub scope: String,
    pub name: String,
}

/// Output payload for `awf workflow list`.
#[derive(Debug)]
pub struct WorkflowListOutput {
    pub workflows: Vec<WorkflowListEntry>,
}

impl fmt::Display for WorkflowListOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.workflows.is_empty() {
            return writeln!(
                f,
                "No workflows found. Create one with: awf workflow new <name>"
            );
        }
        let width = self
            .workflows
            .iter()
            .map(|w| w.scope.len() + 1 + w.name.len())
            .max()
            .unwrap_or(0)
            .max("WORKFLOW".len());
        writeln!(f, "WORKFLOW")?;
        writeln!(f, "{}", "─".repeat(width))?;
        for w in &self.workflows {
            writeln!(f, "{}:{}", w.scope, w.name.blue())?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf workflow list`.
///
/// Lists all workflow definitions across all scopes in `scope:name` format.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no project root can be
/// found.
pub fn list() -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let listings = fs::list_workflows(&entries)?;

    let output = WorkflowListOutput {
        workflows: listings
            .into_iter()
            .map(|l| WorkflowListEntry { scope: l.scope, name: l.name })
            .collect(),
    };
    print_output(&output);
    Ok(())
}
