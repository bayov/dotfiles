//! Error type and `Result` alias for the entire `awf` crate.

use thiserror::Error;

/// All errors that can occur within `awf`.
///
/// Marked `#[non_exhaustive]` so that downstream code (integration tests,
/// future library consumers) must use a catch-all `_` arm, letting us add
/// variants without breaking them.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An identifier string did not match the allowed pattern
    /// (`[a-zA-Z0-9_/\-]+`).
    #[error("invalid id {id:?}: {reason}")]
    InvalidId { id: String, reason: String },

    /// No `.agents/` directory was found by walking up the directory tree.
    #[error("no .agents/ directory found starting from {start:?}")]
    ProjectRootNotFound { start: std::path::PathBuf },

    /// A flow path could not be determined (no `--flow` flag and no
    /// `AWF_ACTIVE_FLOW` environment variable).
    #[error("no flow specified: pass --flow=<path> or set AWF_ACTIVE_FLOW")]
    FlowPathRequired,

    /// A workflow was referenced in multiple scopes without disambiguation.
    #[error("ambiguous workflow {name:?}: found in multiple scopes ({scopes})")]
    AmbiguousWorkflow { name: String, scopes: String },

    /// A workflow was not found in any search entry.
    #[error("workflow {name:?} not found")]
    WorkflowNotFound { name: String },

    /// A workflow file could not be parsed.
    #[error("failed to parse workflow {name:?}: {source}")]
    WorkflowParseFailed {
        name: String,
        #[source]
        source: toml::de::Error,
    },

    /// Attempted to create a workflow that already exists.
    #[error("workflow {name:?} already exists")]
    WorkflowAlreadyExists { name: String },

    /// A flow state file was not found at the given directory.
    #[error("flow not found at {path:?}")]
    FlowNotFound { path: std::path::PathBuf },

    /// A flow state file could not be parsed.
    #[error("failed to parse flow state at {path:?}: {source}")]
    FlowParseFailed {
        path: std::path::PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// Attempted to create a flow that already exists.
    #[error("flow already exists at {path:?}")]
    FlowAlreadyExists { path: std::path::PathBuf },

    /// The workflow has only one stage and `advance_stage` was called while
    /// already on the last stage.
    #[error("already on the last stage; cannot advance further")]
    AlreadyOnLastStage,

    /// The current stage is not marked as `skippable` in the workflow.
    #[error("stage {stage:?} is not marked as skippable in the workflow")]
    StageNotSkippable { stage: String },

    /// A workflow definition violated a structural invariant (e.g. mutual
    /// exclusivity of fields, duplicate stage or step names).
    #[error("workflow validation failed: {message}")]
    WorkflowValidation { message: String },

    /// A task list TOML file was not found at the given path.
    #[error("task list not found at {path:?}")]
    TaskListNotFound { path: std::path::PathBuf },

    /// A task list TOML file could not be parsed.
    #[error("failed to parse task list at {path:?}: {source}")]
    TaskListParseFailed {
        path: std::path::PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// A generic I/O error wrapping `std::io::Error`.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A serialisation error wrapping `toml::ser::Error`.
    #[error("TOML serialisation error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// A config file could not be parsed.
    #[error("failed to parse config at {path:?}: {source}")]
    ConfigParseFailed {
        path: std::path::PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// Convenience alias so callers can write `Result<T>` instead of
/// `std::result::Result<T, crate::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
