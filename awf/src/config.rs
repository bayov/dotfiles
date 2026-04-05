//! Configuration loading for `awf`.
//!
//! Merges user-level (`~/.config/awf.toml`) and project-level
//! (`.config/awf.toml`) config files, with `AWF_*` environment variables
//! taking highest priority.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crate::result::Error;
use crate::result::Result;
use crate::store::resolve::expand_tilde;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Per-scope workflow configuration.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ScopeConfig {
    /// Override the default directory for this scope's workflows.
    pub workflow_dir: Option<String>,
}

/// Resolved, merged configuration from all sources.
#[derive(Debug, Default)]
pub struct Config {
    /// Active flow directory path (tilde-expanded), if set.
    pub active_flow: Option<PathBuf>,

    /// Root directories to scan recursively for flow state files
    /// (tilde-expanded).
    pub flow_search_dirs: Vec<PathBuf>,

    /// Named scope configurations (custom scopes, or overrides for `user`).
    pub scopes: HashMap<String, ScopeConfig>,

    /// Repo-wide named shell commands available to all workflows.
    ///
    /// Defined under `[commands]` in `.config/awf.toml`. Workflow-level
    /// `[commands]` tables take precedence when the same alias is defined in
    /// both places.
    pub commands: HashMap<String, String>,
}

/// A single entry in the ordered list of directories to search for workflows.
pub struct WorkflowSearchEntry {
    /// The directory that contains workflow subdirectories.
    pub dir: PathBuf,

    /// The scope name (e.g. `"project"`, `"user"`, or a custom name).
    pub scope: String,
}

// ---------------------------------------------------------------------------
// Config file path
// ---------------------------------------------------------------------------

/// Returns the path to the user-level config file.
///
/// Uses `$XDG_CONFIG_HOME/awf.toml` if `XDG_CONFIG_HOME` is set, otherwise
/// `~/.config/awf.toml`.
pub fn user_config_path() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME").map_or_else(
        |_| expand_tilde("~/.config/awf.toml"),
        |xdg| PathBuf::from(xdg).join("awf.toml"),
    )
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Raw config as deserialised from a single TOML file.
#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    pub active_flow: Option<String>,
    #[serde(default)]
    pub flow_search_dirs: Vec<String>,
    #[serde(default)]
    pub scopes: HashMap<String, ScopeConfig>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
}

/// Reads and deserialises a config file, returning the default if absent.
///
/// Returns an error only if the file exists but cannot be parsed.
fn load_raw(path: &Path) -> Result<RawConfig> {
    if !path.is_file() {
        return Ok(RawConfig::default());
    }
    let contents = std::fs::read_to_string(path).map_err(Error::Io)?;
    toml::from_str(&contents).map_err(|source| Error::ConfigParseFailed {
        path: path.to_path_buf(),
        source,
    })
}

/// Loads and merges configuration from all sources.
///
/// Priority order (lowest to highest):
/// 1. User config (`~/.config/awf.toml` or `$XDG_CONFIG_HOME/awf.toml`)
/// 2. Project config (`<project_root>/.config/awf.toml`)
/// 3. `AWF_*` environment variables
///
/// `active_flow`: project overrides user; env var overrides both.
/// `flow_search_dirs`: concatenated (user list + project list).
/// `scopes`: user is the base; project entries override same-named scopes.
pub fn load(project_root: &Path) -> Result<Config> {
    let user_raw = load_raw(&user_config_path())?;
    let project_raw = load_raw(&project_root.join(".config/awf.toml"))?;

    // active_flow: user < project < AWF_ACTIVE_FLOW env var
    let active_flow = std::env::var("AWF_ACTIVE_FLOW")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| project_raw.active_flow.clone())
        .or_else(|| user_raw.active_flow.clone())
        .map(|s| expand_tilde(&s));

    // flow_search_dirs: concatenate user + project, tilde-expand each
    let flow_search_dirs: Vec<PathBuf> = user_raw
        .flow_search_dirs
        .iter()
        .chain(project_raw.flow_search_dirs.iter())
        .map(|s| expand_tilde(s))
        .collect();

    // scopes: user as base, project entries override same-named scopes
    let mut scopes = user_raw.scopes;
    for (k, v) in project_raw.scopes {
        scopes.insert(k, v);
    }

    // commands: user as base, project entries override same-named aliases
    let mut commands = user_raw.commands;
    for (k, v) in project_raw.commands {
        commands.insert(k, v);
    }

    Ok(Config { active_flow, flow_search_dirs, scopes, commands })
}

// ---------------------------------------------------------------------------
// Workflow search entries
// ---------------------------------------------------------------------------

/// Returns the ordered list of workflow search entries.
///
/// Built-in entries (always present):
/// - `project`: `<project_root>/.agents/workflows`
/// - `user`: `~/.agents/workflows` (override via `[scopes.user]`)
///
/// Custom scopes from config follow, in `HashMap` iteration order.
pub fn effective_workflow_entries(
    project_root: &Path,
    config: &Config,
) -> Vec<WorkflowSearchEntry> {
    let mut entries = Vec::new();

    // project scope
    entries.push(WorkflowSearchEntry {
        dir: project_root.join(".agents/workflows"),
        scope: "project".to_string(),
    });

    // user scope: config override or default ~/.agents/workflows
    let user_dir = config
        .scopes
        .get("user")
        .and_then(|s| s.workflow_dir.as_deref())
        .map_or_else(|| expand_tilde("~/.agents/workflows"), expand_tilde);
    entries
        .push(WorkflowSearchEntry { dir: user_dir, scope: "user".to_string() });

    // custom scopes (skip built-in names)
    for (scope, scope_config) in &config.scopes {
        if scope == "user" || scope == "project" {
            continue;
        }
        if let Some(dir) = &scope_config.workflow_dir {
            entries.push(WorkflowSearchEntry {
                dir: expand_tilde(dir),
                scope: scope.clone(),
            });
        }
    }

    entries
}
