//! Filesystem I/O for workflow definitions stored under `.awf/workflows/`.

use std::path::Path;

use tracing::instrument;

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
