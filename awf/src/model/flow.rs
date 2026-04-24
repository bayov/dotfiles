//! Flow state types: [`FlowState`] and [`StageHistoryEntry`].

use std::collections::HashMap;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// The outcome of a completed stage event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Completed,
    Failed,
    ForceCompleted,
    Skipped,
}

/// A record of a single completed stage event within a flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageHistoryEntry {
    pub stage: String,
    pub time: DateTime<Utc>,
    pub status: StageStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

// ---------------------------------------------------------------------------
// Task runner types
// ---------------------------------------------------------------------------

/// The lifecycle status of a task list within a task-runner stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskListStatus {
    /// Not yet started; all `depends_on` lists are complete (or there are
    /// none).
    Pending,
    /// At least one task has been started but the list is not yet complete.
    InProgress,
    /// All tasks have passed or been skipped; the list was advanced.
    Complete,
    /// One or more `depends_on` lists are not yet complete.
    Blocked,
}

/// The type of a task for logging purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskLogKind {
    Agent,
    Command,
}

/// The outcome of a single task execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskLogStatus {
    Pass,
    Fail,
    Skipped,
}

/// A log entry recording the outcome of a single task execution.
///
/// Appended to [`FlowState::task_log`] by the task CLI commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskLogEntry {
    /// Name of the task list this task belongs to (e.g. `"foo"`).
    pub list: String,
    /// Task ID within the list (e.g. `"1.1"`).
    pub task_id: String,
    /// Name of the task-runner stage this task belongs to.
    pub stage: String,
    /// Whether this was an agent or command task.
    pub kind: TaskLogKind,
    /// Outcome of the task execution.
    pub status: TaskLogStatus,
    /// When the task was completed (agent tasks) or started (command tasks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Wall-clock duration for command tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Exit code for command tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Human-readable reason for skipped tasks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// FlowState
// ---------------------------------------------------------------------------

/// The persisted state of a single flow — a running instance of a workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowState {
    pub id: String,
    pub workflow: String,
    pub workflow_version: u32,
    pub current_stage: String,
    pub stage_started_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived: bool,

    /// Ordered log of completed stage events (pass, fail, force, skip).
    #[serde(default)]
    pub stage_history: Vec<StageHistoryEntry>,

    /// Per-task-list status for the current task-runner stage (if any).
    ///
    /// Maps task list path (e.g. `"tasks/foo.toml"`) → [`TaskListStatus`].
    /// Only populated for flows that have entered a task-runner stage.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub task_runner_state: HashMap<String, TaskListStatus>,

    /// Log of individual task completions within task-runner stages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_log: Vec<TaskLogEntry>,
}

#[cfg(test)]
#[path = "flow.tests.rs"]
mod tests;
