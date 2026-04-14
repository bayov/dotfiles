//! Resolving the project root and active flow path.

use std::path::Path;
use std::path::PathBuf;

use tracing::instrument;

use crate::config::Config;
use crate::result::Error;
use crate::result::Result;

// ---------------------------------------------------------------------------
// Template expansion
// ---------------------------------------------------------------------------

/// Error returned by [`expand_template`] when a template cannot be resolved.
#[derive(Debug)]
pub enum TemplateError {
    /// A `{variable}` placeholder was not recognised.
    UnknownVariable(String),
    /// A `{` was opened but never closed.
    UnclosedBrace,
    /// `{project_name}` was used but `project_name` is not set in the project
    /// config.
    ProjectNameNotConfigured,
    /// `{project_root}` was used but no project root could be determined.
    ProjectRootNotConfigured,
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVariable(name) => {
                write!(f, "unknown template variable `{{{name}}}`")
            }
            Self::UnclosedBrace => write!(f, "unclosed `{{` in path template"),
            Self::ProjectNameNotConfigured => write!(
                f,
                "`{{project_name}}` requires `project_name` to be set in \
                 `.config/awf.toml`"
            ),
            Self::ProjectRootNotConfigured => write!(
                f,
                "`{{project_root}}` requires a project root \
                 (`.config/awf.toml`, `.agents/`, `.jj/`, or `.git/` \
                 directory)"
            ),
        }
    }
}

impl std::error::Error for TemplateError {}

/// Expands template variables inside `path` and returns the result.
///
/// # Supported variables
///
/// | Syntax              | Expands to                                      |
/// |---------------------|-------------------------------------------------|
/// | `{project_root}`    | Absolute path to the project root directory     |
/// | `{project_name}`    | Value of `project_name` in `.config/awf.toml`  |
/// | `{time:FORMAT}`     | Current local time formatted by `FORMAT`        |
///
/// ## Time format tokens
///
/// `YYYY` → year (4-digit), `MM` → month, `DD` → day,
/// `HH` → hour (24 h), `mm` → minute, `ss` → second.
///
/// ## Escaping
///
/// `\{` and `\}` are replaced with literal `{` and `}` without triggering
/// variable expansion.
///
/// # Errors
///
/// - [`TemplateError::ProjectRootNotConfigured`] — `{project_root}` used but
///   `project_root` is `None`.
/// - [`TemplateError::ProjectNameNotConfigured`] — `{project_name}` used but
///   `project_name` is `None`.
/// - [`TemplateError::UnknownVariable`] — unrecognised `{variable}`.
/// - [`TemplateError::UnclosedBrace`] — `{` with no matching `}`.
pub fn expand_template(
    path: &str,
    project_name: Option<&str>,
    project_root: Option<&Path>,
) -> std::result::Result<String, TemplateError> {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek().copied() {
                Some('{' | '}') => result.push(chars.next().expect("peeked")),
                _ => result.push('\\'),
            }
        } else if c == '{' {
            // Collect variable name (and optional format) up to the closing
            // '}'.
            let mut var = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '}' {
                    closed = true;
                    break;
                }
                var.push(inner);
            }
            if !closed {
                return Err(TemplateError::UnclosedBrace);
            }
            if let Some((name, fmt)) = var.split_once(':') {
                match name {
                    "time" => {
                        let strftime = user_fmt_to_strftime(fmt);
                        let now = chrono::Local::now();
                        result.push_str(&now.format(&strftime).to_string());
                    }
                    _ => return Err(TemplateError::UnknownVariable(var)),
                }
            } else {
                match var.as_str() {
                    "project_root" => match project_root {
                        Some(p) => result.push_str(&p.to_string_lossy()),
                        None => {
                            return Err(
                                TemplateError::ProjectRootNotConfigured,
                            );
                        }
                    },
                    "project_name" => match project_name {
                        Some(name) => result.push_str(name),
                        None => {
                            return Err(
                                TemplateError::ProjectNameNotConfigured,
                            );
                        }
                    },
                    _ => return Err(TemplateError::UnknownVariable(var)),
                }
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

/// Converts the user-facing date/time `FORMAT` string to a chrono strftime
/// format string.
///
/// Token mapping:
/// `YYYY`→`%Y`, `MM`→`%m`, `DD`→`%d`, `HH`→`%H`, `mm`→`%M`, `ss`→`%S`.
fn user_fmt_to_strftime(fmt: &str) -> String {
    fmt.replace("YYYY", "%Y")
        .replace("MM", "%m")
        .replace("DD", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
}

// ---------------------------------------------------------------------------
// Project root
// ---------------------------------------------------------------------------

/// Walks up the directory tree from `start` to find the nearest directory
/// that looks like a project root.
///
/// At each level the following markers are checked in priority order:
///
/// 1. `.config/awf.toml` — explicit awf project config
/// 2. `.agents/` directory — awf convention
/// 3. `.jj/` directory — Jujutsu VCS root
/// 4. `.git/` directory — Git VCS root
///
/// The first directory (starting from `start`, walking toward the filesystem
/// root) that contains any of these markers is returned.  If `start` is not
/// a directory the walk begins at its parent.
///
/// # Errors
///
/// Returns [`Error::ProjectRootNotFound`] when no marker is found in `start`
/// or any of its ancestors.
#[instrument]
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    assert_safe_path(start);
    let mut current = start;

    loop {
        if current.join(".config/awf.toml").is_file()
            || current.join(".agents").is_dir()
            || current.join(".jj").is_dir()
            || current.join(".git").is_dir()
        {
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

// ---------------------------------------------------------------------------
// Test-time filesystem safety guard
// ---------------------------------------------------------------------------

/// Asserts that `path` is located under `/tmp` when running tests.
///
/// Prevents accidental reads or writes to real files on the host system during
/// unit or integration tests. Compiled to a no-op in production builds.
#[allow(clippy::missing_const_for_fn)]
fn assert_safe_path(path: &Path) {
    #[cfg(test)]
    assert!(
        path.starts_with("/tmp"),
        "test accessed path outside /tmp: {}",
        path.display(),
    );
    #[cfg(not(test))]
    let _ = path;
}

#[cfg(test)]
#[path = "resolve.tests.rs"]
mod tests;
