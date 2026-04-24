//! Task list types: [`TaskList`] and [`Task`].
//!
//! A task list is a TOML artifact produced by an agent stage and consumed by
//! a `task-runner` stage. It defines an ordered sequence of tasks (agent or
//! command) and optional inter-list dependencies via [`TaskList::depends_on`].

use serde::Deserialize;
use serde::Serialize;

/// A structured list of tasks produced by an agent stage.
///
/// Stored as a TOML file under `tasks/` in the flow's artifact directory.
/// The task-runner stage reads these files to drive execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskList {
    /// Unique name for this task list within the task-runner stage.
    pub name: String,

    /// Human-readable description of what this list implements.
    pub description: String,

    /// Names of other task lists that must be `complete` before this list can
    /// be dispatched. Defaults to empty (no dependencies).
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Ordered tasks in this list. Executed sequentially.
    #[serde(default)]
    pub tasks: Vec<Task>,
}

/// A single task within a [`TaskList`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier within the list (e.g. `"1.1"`, `"2.3"`).
    pub id: String,

    /// Human-readable description of what this task does.
    pub description: String,

    /// The kind-specific payload for this task.
    #[serde(flatten)]
    pub kind: TaskKind,
}

/// The kind of a [`Task`].
///
/// Serializes with an internally-tagged representation: `type` selects the
/// variant and additional fields carry the payload:
///
/// ```toml
/// [[tasks]]
/// id = "1.1"
/// description = "Scaffold the auth module"
/// type = "agent"
///
/// [[tasks]]
/// id = "1.3"
/// description = "Run formatter"
/// type = "command"
/// command = "format"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TaskKind {
    /// An agent-executed task. The agent reads the task description and acts.
    Agent,

    /// A shell command task. Runs the command referenced by `command` (a key
    /// in the workflow-level `[commands]` table).
    Command {
        /// Key into the workflow's `[commands]` table.
        command: String,
    },
}

#[cfg(test)]
#[path = "task_list.tests.rs"]
mod tests;
