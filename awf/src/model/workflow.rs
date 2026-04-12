//! Workflow definition types: [`Workflow`] and [`Stage`].
//!
//! A workflow is a named sequence of stages. Each stage is a single action —
//! either a shell command or an agent invocation — identified by a `kind` field
//! with kind-specific arguments in an `args` table. Workflow files are stored
//! as TOML on disk and deserialized via serde.
//!
//! After deserialisation, call [`Workflow::validate`] to enforce invariants
//! that cannot be expressed in the type system alone (e.g. mutual exclusivity
//! of `command`/`run` and `prompt`/`prompt_file`).

use std::collections::HashMap;
use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;
use tracing::instrument;

use crate::result::Error;
use crate::result::Result;

/// A complete workflow definition, typically loaded from a `.awf/workflows/`
/// TOML file.
///
/// After deserializing, always call [`Workflow::validate`] to check structural
/// invariants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workflow {
    /// Human-readable name of the workflow (e.g. `"feature"`).
    pub name: String,

    /// Schema / revision version for the workflow file format.
    pub version: u32,

    /// Optional short description of the workflow's purpose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Named shell commands that can be referenced by [`StageKind::Command`]
    /// stages via the `command` key.
    ///
    /// Keys are short aliases (e.g. `"lint"`); values are the shell strings
    /// (e.g. `"cargo clippy -- -D warnings"`).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub commands: HashMap<String, String>,

    /// Ordered list of stages that make up the workflow.
    #[serde(default)]
    pub stages: Vec<Stage>,

    /// Suggested path prefix for new flows created from this workflow.
    ///
    /// When set, tooling may use this as the default parent directory when
    /// constructing a flow path. Users are free to override it via `--flow`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_flow_dir: Option<String>,
}

/// A single stage within a [`Workflow`].
///
/// Each stage is a single action identified by its [`StageKind`]. The `kind`
/// field selects the action type and `args` carries the kind-specific payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    /// Unique name for this stage within its workflow (e.g. `"implement"`).
    pub name: String,

    /// Optional short description of what this stage does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// When `true`, the stage can be skipped entirely via `next-stage --skip`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skippable: bool,

    /// Artifact paths this stage produces and makes available to later stages.
    ///
    /// Defaults to an empty list.
    #[serde(default)]
    pub produces: Vec<String>,

    /// The kind-specific payload for this stage.
    #[serde(flatten)]
    pub kind: StageKind,
}

/// The kind-specific payload of a [`Stage`].
///
/// Serializes with an adjacently-tagged representation: `kind` selects the
/// variant and `args` carries the payload:
///
/// ```toml
/// [[stages]]
/// name = "lint"
/// kind = "command"
/// args = { command = "lint" }
///
/// [[stages]]
/// name = "write-design"
/// kind = "agent"
/// args = { prompt = "design.md" }
/// ```
///
/// After deserialisation, validate mutual exclusivity with
/// [`Workflow::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "args", rename_all = "lowercase")]
pub enum StageKind {
    /// A shell command stage.
    ///
    /// Exactly one of `command` (an alias into [`Workflow::commands`]) or
    /// `run` (an inline shell string) must be set. [`Workflow::validate`]
    /// enforces this constraint.
    Command {
        /// Key into [`Workflow::commands`] that resolves to the shell string
        /// to run.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,

        /// Inline shell string to execute directly (alternative to `command`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        run: Option<String>,
    },

    /// An agent invocation stage.
    ///
    /// Exactly one of `prompt` (an inline instruction string) or `prompt_file`
    /// (a path to a prompt file, absolute or relative to the workflow
    /// directory) must be set. [`Workflow::validate`] enforces this
    /// constraint.
    Agent {
        /// Inline instruction string passed directly to the agent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,

        /// Path to a prompt file that provides the agent's task description.
        /// An absolute path is used as-is; a relative path is resolved from
        /// the workflow directory.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt_file: Option<String>,
    },
}

impl Workflow {
    /// Validates the structural invariants of this workflow.
    ///
    /// Checks performed:
    ///
    /// - Stage names are unique within the workflow.
    /// - Every [`StageKind::Command`] stage has exactly one of `command` or
    ///   `run` set (not both, not neither).
    /// - Every [`StageKind::Agent`] stage has exactly one of `prompt` or
    ///   `prompt_file` set (not both, not neither).
    ///
    /// # Errors
    ///
    /// Returns [`Error::WorkflowValidation`] with a descriptive message when
    /// any invariant is violated.
    #[instrument(skip(self), fields(workflow.name = %self.name))]
    pub fn validate(&self) -> Result<()> {
        self.validate_stage_names_unique()?;

        for stage in &self.stages {
            validate_stage_kind(stage)?;
        }

        Ok(())
    }

    /// Checks that all stage names within this workflow are distinct.
    fn validate_stage_names_unique(&self) -> Result<()> {
        let mut seen = HashSet::new();

        for stage in &self.stages {
            if !seen.insert(stage.name.as_str()) {
                return Err(Error::WorkflowValidation {
                    message: format!(
                        "duplicate stage name {:?} in workflow {:?}",
                        stage.name, self.name
                    ),
                });
            }
        }

        Ok(())
    }
}

/// Validates the mutual-exclusivity constraints on a single [`Stage`]'s kind.
///
/// - [`StageKind::Command`]: exactly one of `command` or `run` must be `Some`.
/// - [`StageKind::Agent`]: exactly one of `prompt` or `prompt_file` must be
///   `Some`.
///
/// # Errors
///
/// Returns [`Error::WorkflowValidation`] when the constraint is violated.
fn validate_stage_kind(stage: &Stage) -> Result<()> {
    match &stage.kind {
        StageKind::Command { command, run } => match (command, run) {
            (Some(_), None) | (None, Some(_)) => Ok(()),
            (Some(_), Some(_)) => Err(Error::WorkflowValidation {
                message: format!(
                    "stage {:?}: command stage must not set both `command` and \
                     `run`",
                    stage.name
                ),
            }),
            (None, None) => Err(Error::WorkflowValidation {
                message: format!(
                    "stage {:?}: command stage must set either `command` or \
                     `run`",
                    stage.name
                ),
            }),
        },
        StageKind::Agent { prompt, prompt_file } => {
            match (prompt, prompt_file) {
                (Some(_), None) | (None, Some(_)) => Ok(()),
                (Some(_), Some(_)) => Err(Error::WorkflowValidation {
                    message: format!(
                        "stage {:?}: agent stage must not set both `prompt` \
                         and `prompt_file`",
                        stage.name
                    ),
                }),
                (None, None) => Err(Error::WorkflowValidation {
                    message: format!(
                        "stage {:?}: agent stage must set either `prompt` or \
                         `prompt_file`",
                        stage.name
                    ),
                }),
            }
        }
    }
}

#[cfg(test)]
#[path = "workflow.tests.rs"]
mod tests;
