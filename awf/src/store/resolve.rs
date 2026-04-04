//! Resolving the `.awf/` workspace root from an arbitrary starting path.

use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use tracing::instrument;

use crate::result::Error;
use crate::result::Result;

/// Walks up the directory tree from `start` to find the nearest `.awf/` root.
///
/// Beginning at `start`, each directory is checked for the presence of a
/// `.awf/` subdirectory. If found, the **parent** directory (the one that
/// contains `.awf/`, not `.awf/` itself) is returned. If `start` is not a
/// directory, the walk begins at its parent.
///
/// The walk proceeds upward one level at a time (via [`Path::parent`]) until
/// either:
/// - A `.awf/` subdirectory is found (success), or
/// - The filesystem root is reached with no `.awf/` found (failure).
///
/// # Errors
///
/// Returns [`Error::AwfRootNotFound`] when no `.awf/` directory exists in
/// `start` or any of its ancestors.
#[instrument]
pub fn find_awf_root(start: &Path) -> Result<PathBuf> {
    let mut current = start;

    loop {
        // Check whether this directory contains a `.awf/` subdirectory.
        let candidate = current.join(".awf");
        if candidate.is_dir() {
            return Ok(current.to_path_buf());
        }

        // Move up one level; if there is no parent we have hit the root.
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(Error::AwfRootNotFound {
                    start: start.to_path_buf(),
                });
            }
        }
    }
}

/// Partial deserialisation target for `.awf/config.toml`.
///
/// Only the fields relevant to state-dir resolution are captured; all other
/// keys are ignored by `serde`.
#[derive(Deserialize)]
struct AwfConfig {
    state_dir: Option<String>,
}

/// Replaces a leading `~` with the value of the `HOME` environment variable.
///
/// If the path does not begin with `~`, it is returned unchanged. If `HOME` is
/// not set the tilde is left in place so that callers receive the raw value
/// rather than a silent failure.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    } else if path == "~"
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home);
    }

    PathBuf::from(path)
}

/// Resolves the state directory for the given `awf_root`.
///
/// The resolution order is:
///
/// 1. **`AWF_STATE_DIR` environment variable** — if set, its value is
///    `~`-expanded and returned immediately.
/// 2. **`.awf/config.toml` `state_dir` field** — if the file
///    `<awf_root>/.awf/config.toml` exists and contains a `state_dir` key, the
///    value is `~`-expanded and returned.
/// 3. **Project-local default** — `<awf_root>/.awf` is returned when neither of
///    the above sources provides a value.
///
/// # Errors
///
/// Returns [`Error::Io`] if the config file exists but cannot be read.
/// Returns [`Error::ConfigParseFailed`] if the config file cannot be parsed as
/// TOML.
#[instrument]
pub fn resolve_state_dir(awf_root: &Path) -> Result<PathBuf> {
    // Priority 1: explicit override via environment variable.
    if let Ok(raw) = std::env::var("AWF_STATE_DIR") {
        return Ok(expand_tilde(&raw));
    }

    // Priority 2: optional `state_dir` field inside `.awf/config.toml`.
    let config_path = awf_root.join(".awf").join("config.toml");
    if config_path.is_file() {
        let contents = std::fs::read_to_string(&config_path)?;
        let config: AwfConfig =
            toml::from_str(&contents).map_err(|source| {
                Error::ConfigParseFailed { path: config_path, source }
            })?;

        if let Some(raw) = config.state_dir {
            return Ok(expand_tilde(&raw));
        }
    }

    // Priority 3: fall back to the project-local `.awf/` directory.
    Ok(awf_root.join(".awf"))
}

/// Resolves the active flow ID from the given sources, in priority order.
///
/// The resolution order is:
///
/// 1. **`flag` argument** — if `Some(id)` is passed, that value is returned
///    immediately (e.g. the value of the `--flow` CLI flag).
/// 2. **`AWF_ACTIVE_FLOW` environment variable** — if set and non-empty, its
///    value is returned.
/// 3. **Error** — if neither source provides a value, [`Error::FlowIdRequired`]
///    is returned.
///
/// # Errors
///
/// Returns [`Error::FlowIdRequired`] when no flow ID can be determined from
/// the flag or the environment.
#[instrument]
pub fn resolve_flow_id(flag: Option<&str>) -> Result<String> {
    // Priority 1: explicit flag value (e.g. --flow <id>).
    if let Some(id) = flag {
        return Ok(id.to_string());
    }

    // Priority 2: active flow set in the environment.
    if let Ok(id) = std::env::var("AWF_ACTIVE_FLOW")
        && !id.is_empty()
    {
        return Ok(id);
    }

    // Priority 3: no source provided a flow ID.
    Err(Error::FlowIdRequired)
}

#[cfg(test)]
#[path = "resolve.tests.rs"]
mod tests;
