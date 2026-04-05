//! Filesystem I/O for workflow definitions and flow state.
//!
//! ## Workflow layout
//!
//! Workflows live under a scope-specific directory. Each workflow is a
//! **subdirectory** named after the workflow, containing a `WORKFLOW.toml`
//! definition and any prompt files:
//!
//! ```text
//! <entry.dir>/
//!   <workflow-name>/
//!     WORKFLOW.toml
//!     design.md
//!     ...
//! ```
//!
//! ## Flow layout
//!
//! A flow is a **directory** chosen by the user. Its state lives at
//! `<flow_dir>/flow_state.toml`; artifacts live under `<flow_dir>/artifacts/`.
//!
//! ```text
//! <flow_dir>/
//!   flow_state.toml
//!   artifacts/
//!     design.md
//! ```

use std::path::Path;
use std::path::PathBuf;

use tracing::instrument;

use crate::config::WorkflowSearchEntry;
use crate::model::flow::FlowState;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::result::Result;

// ---------------------------------------------------------------------------
// Workflow types
// ---------------------------------------------------------------------------

/// A lightweight listing entry for a discovered workflow.
pub struct WorkflowListing {
    /// The workflow's short name (directory name under the scope directory).
    pub name: String,

    /// The scope this workflow belongs to (e.g. `"project"`, `"user"`).
    pub scope: String,
}

// ---------------------------------------------------------------------------
// Flow summary type
// ---------------------------------------------------------------------------

/// A lightweight summary of a flow, used for listing.
pub struct FlowSummary {
    /// The full path to the flow directory.
    pub path: PathBuf,

    /// The name of the workflow this flow is an instance of.
    pub workflow: String,

    /// The name of the stage the flow is currently executing.
    pub current_stage: String,

    /// Whether the flow has been archived.
    pub archived: bool,
}

// ---------------------------------------------------------------------------
// Workflow functions (search-entry based)
// ---------------------------------------------------------------------------

/// Lists all workflows found across the given search entries.
///
/// Scans each `entry.dir` for subdirectories that contain a `WORKFLOW.toml`
/// file. Results are sorted by scope then name.
///
/// # Errors
///
/// Returns [`Error::Io`] if reading a directory fails for reasons other than
/// the directory not existing (e.g. permission denied).
pub fn list_workflows(
    entries: &[WorkflowSearchEntry],
) -> Result<Vec<WorkflowListing>> {
    let mut listings = Vec::new();

    for entry in entries {
        let read_dir = match std::fs::read_dir(&entry.dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(Error::Io(e)),
        };

        for de in read_dir {
            let de = de.map_err(Error::Io)?;
            if !de.file_type().map_err(Error::Io)?.is_dir() {
                continue;
            }
            if de.path().join("WORKFLOW.toml").exists()
                && let Some(name) = de.file_name().to_str()
            {
                listings.push(WorkflowListing {
                    name: name.to_string(),
                    scope: entry.scope.clone(),
                });
            }
        }
    }

    listings.sort_by(|a, b| a.scope.cmp(&b.scope).then(a.name.cmp(&b.name)));
    Ok(listings)
}

/// Loads a workflow by name reference from the given search entries.
///
/// `name_ref` is either a bare name (`"feature"`) or a scope-qualified name
/// (`"project:feature"`). A bare name is an error if the same name appears
/// in multiple scopes; use the `"scope:name"` form to disambiguate.
///
/// # Errors
///
/// - [`Error::WorkflowNotFound`] — no matching workflow found.
/// - [`Error::AmbiguousWorkflow`] — bare name found in multiple scopes.
/// - [`Error::WorkflowParseFailed`] — `WORKFLOW.toml` cannot be parsed.
/// - [`Error::Io`] — unexpected I/O failure.
pub fn load_workflow(
    entries: &[WorkflowSearchEntry],
    name_ref: &str,
) -> Result<Workflow> {
    let (scope_filter, bare_name) = parse_name_ref(name_ref);

    let mut found: Vec<PathBuf> = Vec::new();
    let mut found_scopes: Vec<&str> = Vec::new();

    for entry in entries {
        if scope_filter.is_some_and(|s| s != entry.scope.as_str()) {
            continue;
        }
        let wf_path = entry.dir.join(bare_name).join("WORKFLOW.toml");
        if wf_path.exists() {
            found.push(wf_path);
            found_scopes.push(&entry.scope);
        }
    }

    match found.len() {
        0 => Err(Error::WorkflowNotFound { name: name_ref.to_string() }),
        1 => {
            let contents =
                std::fs::read_to_string(&found[0]).map_err(Error::Io)?;
            toml::from_str(&contents).map_err(|source| {
                Error::WorkflowParseFailed {
                    name: name_ref.to_string(),
                    source,
                }
            })
        }
        _ => {
            let scopes = found_scopes.join(", ");
            Err(Error::AmbiguousWorkflow {
                name: bare_name.to_string(),
                scopes,
            })
        }
    }
}

/// Loads a prompt file from a workflow's directory.
///
/// Searches the entries for the workflow identified by `workflow_name_ref`
/// (same format as [`load_workflow`]), then reads
/// `<entry.dir>/<workflow_name>/<filename>`.
///
/// # Errors
///
/// - [`Error::Io`] — file not found or unreadable.
pub fn load_prompt_file(
    entries: &[WorkflowSearchEntry],
    workflow_name_ref: &str,
    filename: &str,
) -> Result<String> {
    let (scope_filter, bare_name) = parse_name_ref(workflow_name_ref);

    for entry in entries {
        if scope_filter.is_some_and(|s| s != entry.scope.as_str()) {
            continue;
        }
        let path = entry.dir.join(bare_name).join(filename);
        if path.exists() {
            return std::fs::read_to_string(&path).map_err(Error::Io);
        }
    }

    Err(Error::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "prompt file {filename:?} not found for workflow \
             {workflow_name_ref:?}"
        ),
    )))
}

/// Creates a workflow template directory under `<agents_root>/workflows/`.
///
/// Creates `<agents_root>/workflows/<name>/WORKFLOW.toml` with stub content.
/// The `workflows/<name>/` directory is created automatically if absent.
///
/// # Errors
///
/// - [`Error::InvalidId`] — `name` contains characters outside the allowed set.
/// - [`Error::WorkflowAlreadyExists`] — `WORKFLOW.toml` already exists.
/// - [`Error::Io`] — unexpected I/O failure.
#[instrument(fields(name = name))]
pub fn create_workflow_template(agents_root: &Path, name: &str) -> Result<()> {
    use crate::model::id::validate_id;

    validate_id(name)?;

    let workflow_dir = agents_root.join("workflows").join(name);
    let wf_toml_path = workflow_dir.join("WORKFLOW.toml");

    if wf_toml_path.exists() {
        return Err(Error::WorkflowAlreadyExists { name: name.to_string() });
    }

    std::fs::create_dir_all(&workflow_dir).map_err(Error::Io)?;

    let contents = build_workflow_template(name);
    std::fs::write(&wf_toml_path, contents).map_err(Error::Io)?;

    Ok(())
}

/// Returns the workflow directory path for a given name reference and entries.
///
/// Used by `validate` to locate prompt files. Returns `None` if the workflow
/// is not found in any entry.
pub fn find_workflow_dir(
    entries: &[WorkflowSearchEntry],
    name_ref: &str,
) -> Option<PathBuf> {
    let (scope_filter, bare_name) = parse_name_ref(name_ref);

    for entry in entries {
        if scope_filter.is_some_and(|s| s != entry.scope.as_str()) {
            continue;
        }
        let dir = entry.dir.join(bare_name);
        if dir.join("WORKFLOW.toml").exists() {
            return Some(dir);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Flow functions (path-based)
// ---------------------------------------------------------------------------

/// Creates a new flow at `flow_dir`.
///
/// Creates the directory tree (`<flow_dir>/artifacts/`), validates the
/// `flow_dir` does not already have a `flow_state.toml`, and writes the initial
/// state.
///
/// The flow `id` is set to the last component of `flow_dir`.
///
/// # Errors
///
/// - [`Error::FlowAlreadyExists`] — `flow_state.toml` already exists at
///   `flow_dir`.
/// - [`Error::TomlSer`] — initial state cannot be serialised.
/// - [`Error::Io`] — unexpected I/O failure.
#[instrument(fields(flow_dir = ?flow_dir, workflow = workflow))]
pub fn create_flow(
    flow_dir: &Path,
    workflow: &str,
    first_stage: &str,
) -> Result<()> {
    use chrono::Utc;

    if flow_dir.join("flow_state.toml").exists() {
        return Err(Error::FlowAlreadyExists { path: flow_dir.to_path_buf() });
    }

    std::fs::create_dir_all(flow_dir.join("artifacts")).map_err(Error::Io)?;

    let id = flow_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let now = Utc::now();
    let state = FlowState {
        id,
        workflow: workflow.to_string(),
        workflow_version: 1,
        current_stage: first_stage.to_string(),
        stage_entered_at: now,
        created_at: now,
        updated_at: now,
        archived: false,
        stage_history: vec![],
        step_log: vec![],
    };

    save_flow(flow_dir, &state)
}

/// Loads the flow state from `<flow_dir>/flow_state.toml`.
///
/// # Errors
///
/// - [`Error::FlowNotFound`] — `flow_state.toml` does not exist.
/// - [`Error::FlowParseFailed`] — `flow_state.toml` cannot be parsed.
/// - [`Error::Io`] — unexpected I/O failure.
#[instrument(fields(flow_dir = ?flow_dir))]
pub fn load_flow(flow_dir: &Path) -> Result<FlowState> {
    let path = flow_dir.join("flow_state.toml");

    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::FlowNotFound { path: flow_dir.to_path_buf() });
        }
        Err(e) => return Err(Error::Io(e)),
    };

    toml::from_str(&contents).map_err(|source| Error::FlowParseFailed {
        path: flow_dir.to_path_buf(),
        source,
    })
}

/// Saves a flow state to `<flow_dir>/flow_state.toml` atomically.
///
/// Writes to a temp file then renames to ensure readers always see a
/// complete, consistent snapshot.
///
/// # Errors
///
/// - [`Error::TomlSer`] — state cannot be serialised.
/// - [`Error::Io`] — unexpected I/O failure.
#[instrument(fields(flow_dir = ?flow_dir))]
pub fn save_flow(flow_dir: &Path, state: &FlowState) -> Result<()> {
    let target_path = flow_dir.join("flow_state.toml");
    let tmp_path = flow_dir.join("flow_state.toml.tmp");

    std::fs::create_dir_all(flow_dir).map_err(Error::Io)?;

    let contents = toml::to_string(state)?;

    std::fs::write(&tmp_path, contents).map_err(Error::Io)?;
    std::fs::rename(&tmp_path, &target_path).map_err(Error::Io)?;

    Ok(())
}

/// Marks a flow as archived by setting `archived = true` in its
/// `flow_state.toml`.
///
/// Unlike the previous model, the flow directory is **not moved** — archiving
/// is purely a flag in the state file.
///
/// # Errors
///
/// - [`Error::FlowNotFound`] — no `flow_state.toml` at `flow_dir`.
/// - [`Error::TomlSer`] — updated state cannot be serialised.
/// - [`Error::Io`] — unexpected I/O failure.
#[instrument(fields(flow_dir = ?flow_dir))]
pub fn archive_flow(flow_dir: &Path) -> Result<()> {
    use chrono::Utc;

    if !flow_dir.join("flow_state.toml").exists() {
        return Err(Error::FlowNotFound { path: flow_dir.to_path_buf() });
    }

    let mut state = load_flow(flow_dir)?;
    state.archived = true;
    state.updated_at = Utc::now();
    save_flow(flow_dir, &state)
}

/// Recursively scans each directory in `search_dirs` for `flow_state.toml`
/// files.
///
/// Each directory containing a `flow_state.toml` is treated as a flow
/// directory. Subdirectories within a flow directory are not recursed into.
///
/// Results are sorted by path.
///
/// # Errors
///
/// Returns [`Error::Io`] if directory iteration fails.
pub fn scan_flows(search_dirs: &[PathBuf]) -> Result<Vec<FlowSummary>> {
    let mut summaries = Vec::new();
    for dir in search_dirs {
        scan_dir_for_flows(dir, &mut summaries)?;
    }
    summaries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(summaries)
}

// ---------------------------------------------------------------------------
// Artifact functions
// ---------------------------------------------------------------------------

/// Copies a file into `<flow_dir>/artifacts/<name>`.
///
/// # Errors
///
/// - [`Error::Io`] — source file unreadable, artifact dir creation failed, or
///   copy failed.
#[instrument(fields(flow_dir = ?flow_dir, name = name, src = ?src))]
pub fn put_artifact(flow_dir: &Path, name: &str, src: &Path) -> Result<()> {
    let artifact_dir = flow_dir.join("artifacts");
    std::fs::create_dir_all(&artifact_dir).map_err(Error::Io)?;
    let target = artifact_dir.join(name);
    std::fs::copy(src, &target).map_err(Error::Io)?;
    Ok(())
}

/// Returns `true` if `<flow_dir>/artifacts/<name>` exists.
#[allow(dead_code)]
pub fn artifact_exists(flow_dir: &Path, name: &str) -> bool {
    flow_dir.join("artifacts").join(name).exists()
}

/// Reads the contents of `<flow_dir>/artifacts/<name>` as a UTF-8 string.
///
/// # Errors
///
/// - [`Error::Io`] — file absent or unreadable.
#[allow(dead_code)]
#[instrument(fields(flow_dir = ?flow_dir, name = name))]
pub fn read_artifact(flow_dir: &Path, name: &str) -> Result<String> {
    let path = flow_dir.join("artifacts").join(name);
    std::fs::read_to_string(&path).map_err(Error::Io)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Recursively scans `dir` for `flow_state.toml` files.
///
/// When a `flow_state.toml` is found in a directory, that directory is treated
/// as a flow directory and is **not** recursed into further.
fn scan_dir_for_flows(
    dir: &Path,
    summaries: &mut Vec<FlowSummary>,
) -> Result<()> {
    // If this directory is itself a flow directory, record it and stop.
    let state_path = dir.join("flow_state.toml");
    if state_path.is_file() {
        if let Ok(contents) = std::fs::read_to_string(&state_path)
            && let Ok(state) = toml::from_str::<FlowState>(&contents)
        {
            summaries.push(FlowSummary {
                path: dir.to_path_buf(),
                workflow: state.workflow,
                current_stage: state.current_stage,
                archived: state.archived,
            });
        }
        // Do not recurse into a flow directory.
        return Ok(());
    }

    // Recurse into subdirectories.
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::Io(e)),
    };

    for entry in read_dir {
        let entry = entry.map_err(Error::Io)?;
        if entry.file_type().map_err(Error::Io)?.is_dir() {
            scan_dir_for_flows(&entry.path(), summaries)?;
        }
    }

    Ok(())
}

/// Parses a workflow name reference into an optional scope filter and a bare
/// name.
///
/// `"project:feature"` → `(Some("project"), "feature")`
/// `"feature"`         → `(None, "feature")`
pub fn parse_name_ref(name_ref: &str) -> (Option<&str>, &str) {
    name_ref.find(':').map_or((None, name_ref), |pos| {
        (Some(&name_ref[..pos]), &name_ref[pos + 1..])
    })
}

/// Renders the stub TOML template for a new workflow named `name`.
fn build_workflow_template(name: &str) -> String {
    format!(
        r#"name = "{name}"
version = 1
description = "TODO: describe this workflow."

# Optional: define reusable shell commands.
# [commands]
# test = "cargo test"
# lint = "cargo clippy -- -D warnings"

[[stages]]
name = "first-stage"
description = "TODO: describe this stage."
# produces = ["artifact.md"]
steps = [
    # {{ name = "write-something", type = "agent", prompt = "first-stage.md" }},
    # {{ name = "run-tests", type = "command", command = "test" }},
]

# [[stages]]
# name = "done"
# description = "Complete."
"#
    )
}

#[cfg(test)]
#[path = "fs.tests.rs"]
mod tests;
