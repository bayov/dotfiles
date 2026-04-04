//! Filesystem I/O for workflow definitions stored under `.awf/workflows/`.

use std::path::Path;

use tracing::instrument;

use crate::model::flow::FlowState;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::result::Result;

/// A lightweight summary of a flow, used for listing flows without loading
/// their full state.
///
/// Returned by [`list_flows`] for each flow found in the state directory.
pub struct FlowSummary {
    /// The unique identifier of the flow (e.g. `"add-oauth"`).
    pub id: String,

    /// The name of the workflow this flow is an instance of (e.g.
    /// `"feature"`).
    pub workflow: String,

    /// The name of the stage the flow is currently executing (e.g.
    /// `"design"`).
    pub current_stage: String,

    /// Whether the flow has been archived.
    pub archived: bool,
}

/// Lists all workflow names defined in the `.awf/workflows/` directory.
///
/// Scans `<awf_root>/workflows/` for files with a `.toml` extension and
/// returns their stems (filenames without the `.toml` suffix), sorted
/// alphabetically.
///
/// If the `workflows/` directory does not exist, an empty list is returned
/// rather than an error — this is the expected state for a freshly
/// initialised `.awf/` root that has no workflows yet.
///
/// # Errors
///
/// - Returns [`Error::Io`] if reading the directory fails for any reason other
///   than the directory not existing (e.g. permission denied).
#[instrument(fields(awf_root = ?awf_root))]
pub fn list_workflows(awf_root: &Path) -> Result<Vec<String>> {
    let workflows_dir = awf_root.join("workflows");

    // If the workflows directory doesn't exist yet, treat it as empty rather
    // than propagating a NotFound error — a fresh .awf root has no workflows.
    let read_dir = match std::fs::read_dir(&workflows_dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(vec![]);
        }
        Err(e) => return Err(Error::Io(e)),
    };

    let mut names = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();

        // Only collect files whose extension is exactly "toml".
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }

        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            names.push(stem.to_string());
        }
    }

    names.sort();
    Ok(names)
}

/// Loads a workflow definition from the `.awf/workflows/` directory.
///
/// The file is expected at `<awf_root>/workflows/<name>.toml`. After reading,
/// the raw TOML is deserialized into a [`Workflow`].
///
/// # Errors
///
/// - Returns [`Error::WorkflowNotFound`] when no file exists at the expected
///   path.
/// - Returns [`Error::WorkflowParseFailed`] when the file exists but cannot be
///   parsed as a valid [`Workflow`].
/// - Returns [`Error::Io`] for any other I/O failure (e.g. permission denied).
#[instrument(fields(name = name))]
pub fn load_workflow(awf_root: &Path, name: &str) -> Result<Workflow> {
    let path = awf_root.join("workflows").join(format!("{name}.toml"));

    // Distinguish "file not found" from other I/O errors so callers receive a
    // descriptive WorkflowNotFound rather than a raw Io error.
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::WorkflowNotFound { name: name.to_string() });
        }
        Err(e) => return Err(Error::Io(e)),
    };

    toml::from_str(&contents).map_err(|source| Error::WorkflowParseFailed {
        name: name.to_string(),
        source,
    })
}

/// Creates a new workflow template file in the `.awf/workflows/` directory.
///
/// Writes a stub TOML file at `<awf_root>/workflows/<name>.toml` with
/// commented-out placeholder sections so the user has a ready-to-edit
/// starting point. The `workflows/` directory is created automatically if
/// it does not yet exist.
///
/// # Errors
///
/// - Returns [`Error::InvalidId`] if `name` contains characters outside the
///   allowed set (`[a-zA-Z0-9_/\-]+`).
/// - Returns [`Error::WorkflowAlreadyExists`] if a file for `name` already
///   exists in the `workflows/` directory.
/// - Returns [`Error::Io`] for any other I/O failure (e.g. permission denied).
#[instrument(fields(name = name))]
pub fn create_workflow_template(awf_root: &Path, name: &str) -> Result<()> {
    use crate::model::id::validate_id;

    // Reject names that violate the identifier rules before touching the
    // filesystem.
    validate_id(name)?;

    let workflows_dir = awf_root.join("workflows");
    let path = workflows_dir.join(format!("{name}.toml"));

    // Guard against silently overwriting an existing workflow definition.
    if path.exists() {
        return Err(Error::WorkflowAlreadyExists { name: name.to_string() });
    }

    // Create the workflows directory only if it is missing; ignore the error
    // if it already exists (that is the common case).
    std::fs::create_dir_all(&workflows_dir).map_err(Error::Io)?;

    let contents = build_workflow_template(name);
    std::fs::write(&path, contents).map_err(Error::Io)?;

    Ok(())
}

/// Creates a new flow in the active flows directory.
///
/// Validates the `flow_id`, checks that no flow with the same ID already
/// exists in either the active or archived directory, creates the flow
/// directory tree (including an `artifacts/` subdirectory), and writes an
/// initial `state.toml`.
///
/// # Errors
///
/// - Returns [`Error::InvalidId`] if `flow_id` contains characters outside the
///   allowed set (`[a-zA-Z0-9_/\-]+`).
/// - Returns [`Error::FlowAlreadyExists`] if a directory for `flow_id` already
///   exists under `<state_dir>/flows/active/` or `<state_dir>/flows/archived/`.
/// - Returns [`Error::TomlSer`] if the initial state cannot be serialised.
/// - Returns [`Error::Io`] for any other I/O failure (directory creation or
///   file write).
#[instrument(fields(flow_id = flow_id, workflow = workflow))]
pub fn create_flow(
    state_dir: &Path,
    flow_id: &str,
    workflow: &str,
    first_stage: &str,
) -> Result<()> {
    use chrono::Utc;

    use crate::model::id::validate_id;

    // Reject IDs that violate the identifier rules before touching the
    // filesystem.
    validate_id(flow_id)?;

    // Guard against creating a flow that already exists in either directory.
    let active_dir = state_dir.join("flows/active").join(flow_id);
    if active_dir.exists() {
        return Err(Error::FlowAlreadyExists { id: flow_id.to_string() });
    }

    let archived_dir = state_dir.join("flows/archived").join(flow_id);
    if archived_dir.exists() {
        return Err(Error::FlowAlreadyExists { id: flow_id.to_string() });
    }

    // Create the flow directory and an empty artifacts/ subdirectory.
    std::fs::create_dir_all(active_dir.join("artifacts")).map_err(Error::Io)?;

    // Build the initial state. Both timestamps are sampled once so that
    // created_at == updated_at on a freshly created flow.
    let now = Utc::now();
    let state = FlowState {
        id: flow_id.to_string(),
        workflow: workflow.to_string(),
        workflow_version: 1,
        current_stage: first_stage.to_string(),
        created_at: now,
        updated_at: now,
        archived: false,
        stage_history: vec![],
        step_log: vec![],
    };

    save_flow(state_dir, &state)?;

    Ok(())
}

/// Saves a flow state to the active flows directory atomically.
///
/// The state is serialized to TOML and written to a temporary file
/// (`state.toml.tmp`) in the same directory as the target, then renamed to
/// `state.toml`. The write-then-rename pattern ensures that any reader of
/// `state.toml` always sees a complete, consistent snapshot — a crash or
/// power loss mid-write leaves the previous `state.toml` intact.
///
/// The target path is `<state_dir>/flows/active/<state.id>/state.toml`. The
/// directory is created automatically if it does not yet exist.
///
/// # Errors
///
/// - Returns [`Error::TomlSer`] if the state cannot be serialized to TOML.
/// - Returns [`Error::Io`] for any I/O failure (directory creation, write, or
///   rename).
#[instrument(fields(flow_id = %state.id))]
pub fn save_flow(state_dir: &Path, state: &FlowState) -> Result<()> {
    let flow_dir = state_dir.join("flows/active").join(&state.id);
    let target_path = flow_dir.join("state.toml");
    let tmp_path = flow_dir.join("state.toml.tmp");

    // Create the directory tree if it does not already exist.
    std::fs::create_dir_all(&flow_dir).map_err(Error::Io)?;

    // Serialize the state to a TOML string before touching the filesystem so
    // that a serialization failure never leaves a partial file on disk.
    let contents = toml::to_string(state)?;

    // Write to the temp file, then atomically rename to the final path so
    // readers always see a complete snapshot.
    std::fs::write(&tmp_path, contents).map_err(Error::Io)?;
    std::fs::rename(&tmp_path, &target_path).map_err(Error::Io)?;

    Ok(())
}

/// Loads a flow state from the active flows directory.
///
/// The file is expected at
/// `<state_dir>/flows/active/<flow_id>/state.toml`. After reading, the raw
/// TOML is deserialized into a [`FlowState`].
///
/// # Errors
///
/// - Returns [`Error::FlowNotFound`] when no file exists at the expected path.
/// - Returns [`Error::FlowParseFailed`] when the file exists but cannot be
///   parsed as a valid [`FlowState`].
/// - Returns [`Error::Io`] for any other I/O failure (e.g. permission denied).
#[instrument(fields(flow_id = flow_id))]
pub fn load_flow(state_dir: &Path, flow_id: &str) -> Result<FlowState> {
    let path = state_dir.join("flows/active").join(flow_id).join("state.toml");

    // Distinguish "file not found" from other I/O errors so callers receive a
    // descriptive FlowNotFound rather than a raw Io error.
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::FlowNotFound { id: flow_id.to_string() });
        }
        Err(e) => return Err(Error::Io(e)),
    };

    toml::from_str(&contents).map_err(|source| Error::FlowParseFailed {
        id: flow_id.to_string(),
        source,
    })
}

/// Lists all flows in the state directory, optionally including archived flows.
///
/// Scans `<state_dir>/flows/active/` for subdirectories; for each, loads the
/// flow state and produces a [`FlowSummary`]. If `include_archived` is `true`,
/// also scans `<state_dir>/flows/archived/` and includes those flows with
/// `archived: true`.
///
/// If either the `active/` or `archived/` directory does not exist, it is
/// treated as empty rather than returning an error — this is expected for a
/// freshly initialised state directory. Results are returned sorted
/// alphabetically by flow id.
///
/// # Errors
///
/// - Returns [`Error::Io`] if reading a directory fails for any reason other
///   than the directory not existing (e.g. permission denied).
/// - Returns [`Error::FlowParseFailed`] if a flow's `state.toml` cannot be
///   parsed.
/// - Returns [`Error::FlowNotFound`] if a flow's `state.toml` is missing after
///   the directory entry was found.
#[instrument(fields(
    state_dir = ?state_dir,
    include_archived = include_archived,
))]
pub fn list_flows(
    state_dir: &Path,
    include_archived: bool,
) -> Result<Vec<FlowSummary>> {
    let mut summaries = Vec::new();

    // Collect summaries from a single flows subdirectory (active or archived).
    // Returns Ok(()) and skips the directory entirely if it does not exist.
    let mut collect_from_dir = |dir: &Path, archived: bool| -> Result<()> {
        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(());
            }
            Err(e) => return Err(Error::Io(e)),
        };

        for entry in read_dir {
            let entry = entry.map_err(Error::Io)?;

            // Only process directory entries; skip any stray files.
            if !entry.file_type().map_err(Error::Io)?.is_dir() {
                continue;
            }

            let flow_id = entry.file_name().into_string().unwrap_or_default();

            // Load state.toml directly from the entry path so we work with
            // both active and archived directories.
            let state_path = entry.path().join("state.toml");
            let contents = match std::fs::read_to_string(&state_path) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Err(Error::FlowNotFound { id: flow_id });
                }
                Err(e) => return Err(Error::Io(e)),
            };

            let state: FlowState =
                toml::from_str(&contents).map_err(|source| {
                    Error::FlowParseFailed { id: flow_id.clone(), source }
                })?;

            summaries.push(FlowSummary {
                id: state.id,
                workflow: state.workflow,
                current_stage: state.current_stage,
                archived,
            });
        }

        Ok(())
    };

    collect_from_dir(&state_dir.join("flows/active"), false)?;

    if include_archived {
        collect_from_dir(&state_dir.join("flows/archived"), true)?;
    }

    summaries.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(summaries)
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
