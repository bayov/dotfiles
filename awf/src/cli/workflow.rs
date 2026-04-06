//! Handlers for `awf workflow` subcommands: `new`, `validate`, and `list`.

use std::env;
use std::fmt;
use std::process;

use owo_colors::OwoColorize as _;
use serde::Serialize;

use crate::cli::output::OutputFormat;
use crate::cli::output::eprint_error;
use crate::cli::output::print_output;
use crate::config::WorkflowSearchEntry;
use crate::result::Error;
use crate::store::fs;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Output payload for `awf workflow new`.
#[derive(Debug, Serialize)]
pub struct WorkflowNewOutput {
    pub path: String,
    pub name: String,
}

impl fmt::Display for WorkflowNewOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} {}", "Created".bright_black(), self.path.bold())?;
        writeln!(
            f,
            "Edit it to define your stages, then run `awf workflow validate {}`.",
            self.name
        )
    }
}

/// Output payload for `awf workflow validate` (success path).
#[derive(Debug, Serialize)]
pub struct WorkflowValidateOutput {
    pub name: String,
    pub status: String,
}

impl fmt::Display for WorkflowValidateOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Workflow {}: {}", self.name, "✓ OK".green().bold())
    }
}

/// A single workflow entry in the list output.
#[derive(Debug, Serialize)]
pub struct WorkflowListEntry {
    pub scope: String,
    pub name: String,
}

/// Output payload for `awf workflow list`.
#[derive(Debug, Serialize)]
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
        writeln!(f, "{}", "WORKFLOW".bold())?;
        writeln!(f, "{}", "─".repeat(width))?;
        for w in &self.workflows {
            writeln!(f, "{}:{}", w.scope, w.name)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// `awf workflow new <name>`
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
pub fn new(name: &str, fmt: &OutputFormat) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
    let agents_root = project_root.join(".agents");

    match fs::create_workflow_template(&agents_root, name) {
        Ok(()) => {
            let output = WorkflowNewOutput {
                path: format!(".agents/workflows/{name}/WORKFLOW.toml"),
                name: name.to_string(),
            };
            print_output(&output, fmt);
            Ok(())
        }
        Err(Error::WorkflowAlreadyExists { name: n }) => {
            eprint_error(format_args!("workflow {n:?} already exists"));
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// `awf workflow validate <name>`
// ---------------------------------------------------------------------------

/// Handles `awf workflow validate [name]`.
///
/// When `name` is given, validates that workflow.  When omitted, discovers and
/// validates every workflow across all scopes.
///
/// For each workflow, loads it, runs structural validation, then checks:
/// 1. Every `command` key referenced in a step exists in `workflow.commands`.
/// 2. Every `prompt` file exists on disk.
///
/// All errors are collected before printing.  Exits with code 1 if any
/// workflow fails validation.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if a workflow cannot be
/// loaded/parsed.
pub fn validate(
    name: Option<&str>,
    fmt: &OutputFormat,
) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
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
        if validate_single(n, &entries, &config.commands, fmt)? {
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
    fmt: &OutputFormat,
) -> color_eyre::Result<bool> {
    let workflow = super::load_workflow_or_exit(entries, name)?;
    let errors =
        super::collect_workflow_errors(&workflow, entries, config_commands);

    if errors.is_empty() {
        let output = WorkflowValidateOutput {
            name: name.to_string(),
            status: "ok".to_string(),
        };
        print_output(&output, fmt);
        Ok(false)
    } else {
        anstream::eprintln!("Workflow {}: {}", name, "✗ FAILED".red().bold());
        for error in &errors {
            anstream::eprintln!("  {} {error}", "·".red());
        }
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// `awf workflow list`
// ---------------------------------------------------------------------------

/// Handles `awf workflow list`.
///
/// Lists all workflow definitions across all scopes in `scope:name` format.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no project root can be
/// found.
pub fn list(fmt: &OutputFormat) -> color_eyre::Result<()> {
    let project_root = super::find_project_root_or_exit(&env::current_dir()?)?;
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
    print_output(&output, fmt);
    Ok(())
}
