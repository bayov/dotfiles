//! Filesystem I/O for workflow definitions stored under `.awf/workflows/`.

use std::path::Path;

use tracing::instrument;

use crate::model::flow::FlowState;
use crate::model::workflow::Workflow;
use crate::result::Error;
use crate::result::Result;

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
