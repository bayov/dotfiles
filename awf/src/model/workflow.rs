//! Workflow definition types: [`Workflow`] and [`Stage`].
//!
//! A workflow is a named sequence of stages. Each stage is a single action
//! (command, agent, or task-runner) identified by its [`StageKind`]. Stage
//! definitions are stored as TOML on disk and deserialized via serde.
//!
//! After deserialisation, call [`Workflow::validate`] to enforce invariants
//! that cannot be expressed in the type system alone.

use std::collections::HashMap;
use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;
use tracing::instrument;

use crate::result::Error;
use crate::result::Result;

/// A complete workflow definition, typically loaded from a `.agents/workflows/`
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

    /// Named shell commands that can be referenced by command stages and
    /// command tasks via the `command` key.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub commands: HashMap<String, String>,

    /// Ordered list of stages that make up the workflow.
    #[serde(default)]
    pub stages: Vec<Stage>,

    /// Suggested path prefix for new flows created from this workflow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_flow_dir: Option<String>,
}

/// A single stage within a [`Workflow`].
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
    #[serde(default)]
    pub produces: Vec<String>,

    /// The kind-specific payload for this stage.
    ///
    /// Uses an untagged serde representation so that task-runner stages (with
    /// a flat `task_lists` field) and linear stages (with `kind`/`args`
    /// fields) can share the same TOML-level struct.
    #[serde(flatten)]
    pub kind: StageKind,
}

/// The kind of a [`Stage`].
///
/// Uses `#[serde(untagged)]` so that task-runner stages can use a flat TOML
/// layout while linear stages keep the existing `kind`/`args` format.
///
/// **Task-runner stage** (flat format):
/// ```toml
/// [[stages]]
/// name = "implement"
/// type = "task-runner"   # conventional; parsed but not required by serde
/// task_lists = ["tasks/foo.toml", "tasks/bar.toml"]
/// parallel = true
/// ```
///
/// **Linear stage** (`kind`/`args` format, unchanged):
/// ```toml
/// [[stages]]
/// name = "lint"
/// kind = "command"
/// args = { command = "lint" }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StageKind {
    /// A task-runner stage that dispatches structured task lists.
    ///
    /// Discriminated by the presence of the `task_lists` field.
    TaskRunner {
        /// Artifact paths (relative to `<flow_dir>/artifacts/`) of the task
        /// list TOML files to dispatch.
        task_lists: Vec<String>,

        /// When `true`, task lists whose `depends_on` are satisfied may be
        /// dispatched simultaneously. Defaults to `false`.
        #[serde(default)]
        parallel: bool,
    },

    /// A linear stage: either a shell command or an agent invocation.
    Linear(LinearKind),
}

/// The kind-specific payload of a linear (non-task-runner) [`Stage`].
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "args", rename_all = "lowercase")]
pub enum LinearKind {
    /// A shell command stage.
    ///
    /// Exactly one of `command` (alias into [`Workflow::commands`]) or `run`
    /// (inline shell string) must be set.
    Command {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        run: Option<String>,
    },

    /// An agent invocation stage.
    ///
    /// Exactly one of `prompt` (inline instruction string) or `prompt_file`
    /// (path to a prompt file) must be set.
    Agent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
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
    /// - Every `command` linear stage has exactly one of `command` or `run`
    ///   set.
    /// - Every `agent` linear stage has exactly one of `prompt` or
    ///   `prompt_file` set.
    /// - Every `task-runner` stage has a non-empty `task_lists` array.
    #[instrument(skip(self), fields(workflow.name = %self.name))]
    pub fn validate(&self) -> Result<()> {
        self.validate_stage_names_unique()?;

        for stage in &self.stages {
            validate_stage_kind(stage)?;
        }

        Ok(())
    }

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

fn validate_stage_kind(stage: &Stage) -> Result<()> {
    match &stage.kind {
        StageKind::TaskRunner { task_lists, .. } => {
            if task_lists.is_empty() {
                return Err(Error::WorkflowValidation {
                    message: format!(
                        "stage {:?}: task-runner stage must have at least one \
                         entry in `task_lists`",
                        stage.name
                    ),
                });
            }
            Ok(())
        }
        StageKind::Linear(linear) => validate_linear_kind(stage, linear),
    }
}

fn validate_linear_kind(stage: &Stage, kind: &LinearKind) -> Result<()> {
    match kind {
        LinearKind::Command { command, run } => match (command, run) {
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
        LinearKind::Agent { prompt, prompt_file } => {
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
