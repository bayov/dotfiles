//! Workflow definition types: [`Workflow`], [`Stage`], and [`Step`].
//!
//! A workflow is a named sequence of stages. Each stage contains one or more
//! steps that are either shell commands or agent invocations. Workflow files
//! are stored as TOML on disk and deserialized via serde.
//!
//! After deserialisation, call [`Workflow::validate`] to enforce invariants
//! that cannot be expressed in the type system alone (e.g. mutual exclusivity
//! of `command`/`run` and `prompt`/`instructions`).

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

    /// Named shell commands that can be referenced by [`StepKind::Command`]
    /// steps via the `command` key.
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
/// Stages group related [`Step`]s and declare which artifacts they require
/// or produce, enabling the runner to enforce ordering and caching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    /// Unique name for this stage within its workflow (e.g. `"implement"`).
    pub name: String,

    /// Optional short description of what this stage does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this stage may be executed more than once in a flow.
    ///
    /// Defaults to `false`.
    #[serde(default)]
    pub repeatable: bool,

    /// Artifact paths this stage produces and makes available to later stages.
    ///
    /// Defaults to an empty list.
    #[serde(default)]
    pub produces: Vec<String>,

    /// Artifact paths this stage requires to have been produced by a prior
    /// stage before it can run.
    ///
    /// Defaults to an empty list.
    #[serde(default)]
    pub requires: Vec<String>,

    /// The ordered list of steps to execute in this stage.
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// A single executable step within a [`Stage`].
///
/// The `name` field identifies the step within its stage. The `type` field in
/// TOML (`"command"` or `"agent"`) selects which variant is used. See
/// [`StepKind`] for the payload carried by each variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// Unique name for this step within its parent stage (e.g. `"run-tests"`).
    pub name: String,

    /// The kind-specific data for this step.
    #[serde(flatten)]
    pub kind: StepKind,
}

/// The kind-specific payload of a [`Step`].
///
/// Serializes with an internally-tagged `type` field
/// (`"command"` or `"agent"`), matching the TOML inline-table format:
///
/// ```toml
/// { name = "lint", type = "command", command = "lint" }
/// { name = "write-design", type = "agent", prompt = "design.md" }
/// ```
///
/// After deserialisation, validate mutual exclusivity with
/// [`Workflow::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StepKind {
    /// A shell command step.
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

    /// An agent invocation step.
    ///
    /// Exactly one of `prompt` (a path to a prompt file) or `instructions`
    /// (an inline instruction string) must be set. [`Workflow::validate`]
    /// enforces this constraint.
    Agent {
        /// Path to a prompt file that provides the agent's task description.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,

        /// Inline instruction string passed directly to the agent (alternative
        /// to `prompt`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instructions: Option<String>,
    },
}

impl Workflow {
    /// Validates the structural invariants of this workflow.
    ///
    /// Checks performed:
    ///
    /// - Stage names are unique within the workflow.
    /// - Step names are unique within each stage.
    /// - Every [`StepKind::Command`] step has exactly one of `command` or `run`
    ///   set (not both, not neither).
    /// - Every [`StepKind::Agent`] step has exactly one of `prompt` or
    ///   `instructions` set (not both, not neither).
    ///
    /// # Errors
    ///
    /// Returns [`Error::WorkflowValidation`] with a descriptive message when
    /// any invariant is violated.
    #[instrument(skip(self), fields(workflow.name = %self.name))]
    pub fn validate(&self) -> Result<()> {
        self.validate_stage_names_unique()?;

        for stage in &self.stages {
            self.validate_step_names_unique(stage)?;

            for step in &stage.steps {
                validate_step_kind(step)?;
            }
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

    /// Checks that all step names within `stage` are distinct.
    fn validate_step_names_unique(&self, stage: &Stage) -> Result<()> {
        let mut seen = HashSet::new();

        for step in &stage.steps {
            if !seen.insert(step.name.as_str()) {
                return Err(Error::WorkflowValidation {
                    message: format!(
                        "duplicate step name {:?} in stage {:?} of workflow {:?}",
                        step.name, stage.name, self.name
                    ),
                });
            }
        }

        Ok(())
    }
}

/// Validates the mutual-exclusivity constraints on a single [`Step`]'s kind.
///
/// - [`StepKind::Command`]: exactly one of `command` or `run` must be `Some`.
/// - [`StepKind::Agent`]: exactly one of `prompt` or `instructions` must be
///   `Some`.
///
/// # Errors
///
/// Returns [`Error::WorkflowValidation`] when the constraint is violated.
fn validate_step_kind(step: &Step) -> Result<()> {
    match &step.kind {
        StepKind::Command { command, run } => match (command, run) {
            (Some(_), None) | (None, Some(_)) => Ok(()),
            (Some(_), Some(_)) => Err(Error::WorkflowValidation {
                message: format!(
                    "step {:?}: command step must not set both `command` and \
                     `run`",
                    step.name
                ),
            }),
            (None, None) => Err(Error::WorkflowValidation {
                message: format!(
                    "step {:?}: command step must set either `command` or `run`",
                    step.name
                ),
            }),
        },
        StepKind::Agent { prompt, instructions } => {
            match (prompt, instructions) {
                (Some(_), None) | (None, Some(_)) => Ok(()),
                (Some(_), Some(_)) => Err(Error::WorkflowValidation {
                    message: format!(
                        "step {:?}: agent step must not set both `prompt` and \
                     `instructions`",
                        step.name
                    ),
                }),
                (None, None) => Err(Error::WorkflowValidation {
                    message: format!(
                        "step {:?}: agent step must set either `prompt` or \
                     `instructions`",
                        step.name
                    ),
                }),
            }
        }
    }
}

#[cfg(test)]
#[path = "workflow.tests.rs"]
mod tests;
