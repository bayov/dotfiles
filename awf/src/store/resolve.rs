//! Resolving the project root and active flow path.

use std::path::Path;
use std::path::PathBuf;

use tracing::instrument;

use crate::config::Config;
use crate::result::Error;
use crate::result::Result;

// ---------------------------------------------------------------------------
// Project root
// ---------------------------------------------------------------------------

/// Walks up the directory tree from `start` to find the nearest directory
/// that contains a `.agents/` subdirectory.
///
/// Beginning at `start`, each directory is checked for a `.agents/`
/// subdirectory. If found, the **parent** directory (i.e. the one that
/// contains `.agents/`, not `.agents/` itself) is returned. If `start` is
/// not a directory, the walk begins at its parent.
///
/// # Errors
///
/// Returns [`Error::ProjectRootNotFound`] when no `.agents/` directory
/// exists in `start` or any of its ancestors.
#[instrument]
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut current = start;

    loop {
        let candidate = current.join(".agents");
        if candidate.is_dir() {
            return Ok(current.to_path_buf());
        }

        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(Error::ProjectRootNotFound {
                    start: start.to_path_buf(),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Flow path resolution
// ---------------------------------------------------------------------------

/// Resolves the active flow directory from the given sources, in priority
/// order.
///
/// The resolution order is:
///
/// 1. **`flag` argument** — if `Some(raw)` is passed, it is tilde-expanded and
///    returned immediately (e.g. the value of the `--flow` CLI flag).
/// 2. **`config.active_flow`** — if the config specifies a flow path, it is
///    returned (already tilde-expanded during config loading).
/// 3. **Error** — if neither source provides a value,
///    [`Error::FlowPathRequired`] is returned.
///
/// # Errors
///
/// Returns [`Error::FlowPathRequired`] when no flow path can be determined.
#[instrument]
pub fn resolve_flow_path(
    flag: Option<&str>,
    config: &Config,
) -> Result<PathBuf> {
    if let Some(raw) = flag {
        return Ok(expand_tilde(raw));
    }

    if let Some(ref path) = config.active_flow {
        return Ok(path.clone());
    }

    Err(Error::FlowPathRequired)
}

// ---------------------------------------------------------------------------
// Tilde expansion
// ---------------------------------------------------------------------------

/// Replaces a leading `~` with the value of the `HOME` environment variable.
///
/// If the path does not begin with `~`, it is returned unchanged. If `HOME`
/// is not set the tilde is left in place so that callers receive the raw
/// value rather than a silent failure.
pub fn expand_tilde(path: &str) -> PathBuf {
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

#[cfg(test)]
#[path = "resolve.tests.rs"]
mod tests;
