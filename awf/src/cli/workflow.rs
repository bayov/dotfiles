//! Handlers for `awf workflow` subcommands: `new`, `validate`, and `list`.

use std::env;
use std::process;

use crate::model::workflow::StepKind;
use crate::result::Error;
use crate::store::fs;
use crate::store::resolve::find_project_root;

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
pub fn new(name: &str) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let agents_root = project_root.join(".agents");

    match fs::create_workflow_template(&agents_root, name) {
        Ok(()) => {
            println!(
                "Created .agents/workflows/{name}/WORKFLOW.toml\n\
                 Edit it to define your stages, then run \
                 `awf workflow validate {name}`."
            );
            Ok(())
        }
        Err(Error::WorkflowAlreadyExists { name: n }) => {
            eprintln!("error: workflow {n:?} already exists");
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
/// Loads the workflow, runs structural validation, then checks:
/// 1. Every `command` key referenced in a step exists in `workflow.commands`.
/// 2. Every `prompt` file exists on disk.
///
/// All errors are collected before printing.
///
/// # Errors
///
/// Returns an error for unexpected I/O failures or if the workflow cannot be
/// loaded/parsed.
pub fn validate(name: &str) -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let workflow = fs::load_workflow(&entries, name)?;

    let mut errors: Vec<String> = Vec::new();

    // Structural validation.
    if let Err(e) = workflow.validate() {
        errors.push(e.to_string());
    }

    // Check 1: every `command` key must exist in workflow.commands.
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

    // Check 2: every `prompt` file must exist on disk.
    let wf_dir = fs::find_workflow_dir(&entries, name);

    if let Some(wf_dir) = wf_dir {
        for stage in &workflow.stages {
            for step in &stage.steps {
                if let StepKind::Agent { prompt: Some(filename), .. } =
                    &step.kind
                    && !wf_dir.join(filename).exists()
                {
                    errors.push(format!(
                        "step {:?} in stage {:?}: prompt file {filename:?} \
                         not found in workflow directory",
                        step.name, stage.name
                    ));
                }
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
pub fn list() -> color_eyre::Result<()> {
    let project_root = find_project_root(&env::current_dir()?)?;
    let config = crate::config::load(&project_root)?;
    let entries =
        crate::config::effective_workflow_entries(&project_root, &config);

    let listings = fs::list_workflows(&entries)?;

    if listings.is_empty() {
        println!(
            "No workflows found. Create one with: awf workflow new <name>"
        );
    } else {
        println!("WORKFLOW");
        println!("--------");
        for listing in &listings {
            println!("{}:{}", listing.scope, listing.name);
        }
    }

    Ok(())
}
