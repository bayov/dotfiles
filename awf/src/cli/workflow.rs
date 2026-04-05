//! Handlers for `awf workflow` subcommands: `new`, `validate`, and `list`.
//!
//! Each public function corresponds to one CLI subcommand and follows the same
//! pattern: locate the `.awf/` root, call into the store layer, and print
//! human-readable output. Errors that the user should act on (e.g. workflow
//! already exists) are printed to stderr and cause a non-zero exit; unexpected
//! I/O or parse errors are propagated up to `main` for rich error reporting via
//! `color_eyre`.

use std::env;
use std::path::Path;
use std::process;

use crate::model::workflow::StepKind;
use crate::result::Error;
use crate::store::fs;
use crate::store::resolve::find_awf_root;

// ---------------------------------------------------------------------------
// `awf workflow new <name>`
// ---------------------------------------------------------------------------

/// Handles `awf workflow new <name>`.
///
/// Locates the nearest `.awf/` root, creates a workflow template file at
/// `.awf/workflows/<name>.toml`, and prints a confirmation message with next
/// steps.
///
/// If a workflow with the given name already exists, a friendly error message
/// is printed to stderr and the process exits with code 1. All other errors are
/// propagated to the caller.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no `.awf/` root can be
/// found.
pub fn new(name: &str) -> color_eyre::Result<()> {
    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");

    match fs::create_workflow_template(&awf_root, name) {
        Ok(()) => {
            println!(
                "Created .awf/workflows/{name}.toml\n\
                 Edit it to define your stages, then run \
                 `awf workflow validate {name}`."
            );
            Ok(())
        }
        Err(Error::WorkflowAlreadyExists { name: n }) => {
            eprintln!(
                "error: workflow {n:?} already exists at \
                       .awf/workflows/{n}.toml"
            );
            process::exit(1);
        }
        Err(e) => Err(e.into()),
    }
}

// ---------------------------------------------------------------------------
// `awf workflow validate <name>`
// ---------------------------------------------------------------------------

/// Handles `awf workflow validate <name>`.
///
/// Loads the workflow TOML, runs its built-in structural validation, then
/// performs two additional checks:
///
/// 1. Every `command` key referenced in a [`StepKind::Command`] step must exist
///    in `workflow.commands`.
/// 2. Every `prompt` file path referenced in a [`StepKind::Agent`] step must
///    exist on disk at `.awf/workflows/<name>/<filename>`.
///
/// All errors are collected before printing so the user sees every problem in
/// a single run. If any errors are found, they are printed to stderr and the
/// process exits with code 1. On success, a brief `OK` message is printed.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if the workflow file cannot
/// be loaded/parsed.
pub fn validate(name: &str) -> color_eyre::Result<()> {
    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");

    let workflow = fs::load_workflow(&awf_root, name)?;

    // Collect all validation errors rather than stopping at the first.
    let mut errors: Vec<String> = Vec::new();

    // Run the built-in structural validation (unique names, step kind
    // mutual-exclusivity, etc.).
    if let Err(e) = workflow.validate() {
        errors.push(e.to_string());
    }

    // Additional check 1: every `command` key must exist in workflow.commands.
    for stage in &workflow.stages {
        for step in &stage.steps {
            if let StepKind::Command { command: Some(key), .. } = &step.kind
                && !workflow.commands.contains_key(key.as_str())
            {
                errors.push(format!(
                    "step {:?} in stage {:?}: command key {key:?} is not \
                     defined in [commands]",
                    step.name, stage.name
                ));
            }
        }
    }

    // Additional check 2: every `prompt` file must exist on disk.
    let prompts_dir = awf_root.join("workflows").join(name);
    for stage in &workflow.stages {
        for step in &stage.steps {
            if let StepKind::Agent { prompt: Some(filename), .. } = &step.kind
                && !prompt_file_exists(&prompts_dir, filename)
            {
                errors.push(format!(
                    "step {:?} in stage {:?}: prompt file {filename:?} \
                     not found at .awf/workflows/{name}/{filename}",
                    step.name, stage.name
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("Workflow {name}: OK");
        Ok(())
    } else {
        for error in &errors {
            eprintln!("  - {error}");
        }
        process::exit(1);
    }
}

/// Returns `true` if the prompt file exists at `<prompts_dir>/<filename>`.
fn prompt_file_exists(prompts_dir: &Path, filename: &str) -> bool {
    prompts_dir.join(filename).exists()
}

// ---------------------------------------------------------------------------
// `awf workflow list`
// ---------------------------------------------------------------------------

/// Handles `awf workflow list`.
///
/// Locates the nearest `.awf/` root, scans the `workflows/` directory, and
/// prints a single-column table of workflow names sorted alphabetically. If no
/// workflows exist, prints a hint on how to create the first one.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if no `.awf/` root can be
/// found.
pub fn list() -> color_eyre::Result<()> {
    let project_root = find_awf_root(&env::current_dir()?)?;
    let awf_root = project_root.join(".awf");

    let names = fs::list_workflows(&awf_root)?;

    if names.is_empty() {
        println!(
            "No workflows found. Create one with: awf workflow new <name>"
        );
    } else {
        // Print a simple single-column table.
        println!("WORKFLOW");
        println!("--------");
        for name in &names {
            println!("{name}");
        }
    }

    Ok(())
}
