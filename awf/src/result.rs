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

    /// No `.awf/` directory was found by walking up the directory tree.
    #[error("no .awf/ root found starting from {start:?}")]
    AwfRootNotFound { start: std::path::PathBuf },

    /// A flow ID could not be determined (no `--flow` flag and no
    /// `AWF_ACTIVE_FLOW` environment variable).
    #[error("a flow ID is required: pass --flow <id> or set AWF_ACTIVE_FLOW")]
    FlowIdRequired,

    /// A workflow file was not found.
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

    /// A flow state file was not found.
    #[error("flow {id:?} not found")]
    FlowNotFound { id: String },

    /// A flow state file could not be parsed.
    #[error("failed to parse flow state for {id:?}: {source}")]
    FlowParseFailed {
        id: String,
        #[source]
        source: toml::de::Error,
    },

    /// Attempted to create a flow that already exists (active).
    #[error("flow {id:?} already exists (active)")]
    FlowAlreadyExists { id: String },

    /// The workflow has only one stage and `advance_stage` was called while
    /// already on the last stage.
    #[error("already on the last stage; cannot advance further")]
    AlreadyOnLastStage,

    /// A generic I/O error wrapping `std::io::Error`.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A serialisation error wrapping `toml::ser::Error`.
    #[error("TOML serialisation error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

/// Convenience alias so callers can write `Result<T>` instead of
/// `std::result::Result<T, crate::Error>`.
pub type Result<T> = std::result::Result<T, Error>;
