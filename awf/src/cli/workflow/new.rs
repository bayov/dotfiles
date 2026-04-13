//! Handler for `awf workflow new`.

use std::env;
use std::fmt;
use std::process;

use owo_colors::OwoColorize as _;

use crate::cli::output::eprint_error;
use crate::cli::output::print_output;
use crate::result::Error;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// Output payload for `awf workflow new`.
#[derive(Debug)]
pub struct WorkflowNewOutput {
    pub path: String,
    pub name: String,
}

impl fmt::Display for WorkflowNewOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Created {}", self.path)?;
        writeln!(
            f,
            "Edit it to define your stages, then run `awf workflow validate {}`.",
            self.name.blue()
        )
    }
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handles `awf workflow new <name>`.
///
/// Locates the nearest project root (`.agents/` marker), creates a workflow
/// template directory at `.agents/workflows/<name>/WORKFLOW.toml`, and prints
/// a confirmation message with next steps.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no project root can be
/// found.
pub fn new(name: &str) -> color_eyre::Result<()> {
    let project_root =
        crate::cli::find_project_root_or_exit(&env::current_dir()?)?;
    let agents_root = project_root.join(".agents");

    match fs::create_workflow_template(&agents_root, name) {
        Ok(()) => {
            let output = WorkflowNewOutput {
                path: format!(".agents/workflows/{name}/WORKFLOW.toml"),
                name: name.to_string(),
            };
            print_output(&output);
            Ok(())
        }
        Err(Error::WorkflowAlreadyExists { name: n }) => {
            eprint_error(format_args!("workflow {n:?} already exists"));
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}
