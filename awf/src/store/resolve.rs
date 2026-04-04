//! Resolving the `.awf/` workspace root from an arbitrary starting path.

use std::path::Path;
use std::path::PathBuf;

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

#[cfg(test)]
#[path = "resolve.tests.rs"]
mod tests;
