//! Flow state types: [`FlowState`], [`StageHistoryEntry`], [`StepLogEntry`],
//! and [`StepStatus`].
//!
//! A flow is a running instance of a workflow. Its state is persisted as a
//! TOML file in `.awf/flows/` and tracks which stage the flow is currently
//! in, the history of stage transitions, and a log of completed step
//! executions.
//!
//! Flow state files are loaded and written verbatim — there is no validation
//! step analogous to [`super::workflow::Workflow::validate`] because the
//! runtime is responsible for maintaining invariants during mutations.

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// The outcome of a completed step execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    /// The step finished successfully (exit code 0 for commands; agent
    /// completed without error for agent steps).
    Pass,

    /// The step finished with a failure (non-zero exit code for commands;
    /// agent returned an error for agent steps).
    Fail,
}

/// A record of a single stage transition within a flow.
///
/// Appended to [`FlowState::stage_history`] whenever the flow advances from
/// one stage to the next. The `exited_at` field is `None` while the stage
/// is still active and is filled in when the flow leaves the stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageHistoryEntry {
    /// The name of the stage (e.g. `"design"`).
    pub stage: String,

    /// The UTC instant at which the flow entered this stage.
    pub entered_at: DateTime<Utc>,

    /// The UTC instant at which the flow exited this stage, or `None` if the
    /// stage is still active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exited_at: Option<DateTime<Utc>>,
}

/// A single completed step execution recorded in a flow's step log.
///
/// Command steps and agent steps share this struct; fields that do not apply
/// to a given step type are stored as `None` and omitted during serialization.
///
/// **Command-specific fields** (present when `step_type` is `"command"`):
/// `exit_code`, `started_at`, `duration_ms`.
///
/// **Agent-specific fields** (present when `step_type` is `"agent"`):
/// `completed_at`.
///
/// The `step_type` field (`type` in TOML) mirrors the `type` tag used in
/// workflow step definitions and allows tooling to distinguish the two kinds
/// of entries without inspecting which optional fields are populated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepLogEntry {
    /// The name of the step as defined in the workflow (e.g. `"format"`).
    pub step: String,

    /// The name of the stage in which this step was executed (e.g.
    /// `"implement"`).
    pub stage: String,

    /// The kind of step: `"command"` or `"agent"` in TOML.
    #[serde(rename = "type")]
    pub step_type: String,

    /// Whether the step passed or failed.
    pub status: StepStatus,

    // -----------------------------------------------------------------------
    // Command-specific fields
    // -----------------------------------------------------------------------
    /// The process exit code (command steps only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    /// The UTC instant at which the command process was started (command steps
    /// only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    /// Wall-clock duration of the command execution in milliseconds (command
    /// steps only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    // -----------------------------------------------------------------------
    // Agent-specific fields
    // -----------------------------------------------------------------------
    /// The UTC instant at which the agent step completed (agent steps only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

/// The persisted state of a single flow — a running instance of a workflow.
///
/// Stored as a TOML file at `.awf/flows/<id>.toml`. The file is the source of
/// truth for all flow progress and is updated in place by the runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowState {
    /// The unique identifier of this flow (e.g. `"add-oauth"`).
    pub id: String,

    /// The name of the workflow this flow is an instance of (e.g.
    /// `"feature"`).
    pub workflow: String,

    /// The version of the workflow definition that was used when this flow was
    /// created. Stored so that a later schema change can be detected.
    pub workflow_version: u32,

    /// The name of the stage the flow is currently executing (e.g.
    /// `"implement"`).
    pub current_stage: String,

    /// The UTC instant at which the flow entered its current stage.
    ///
    /// Recorded when the flow is created (for the initial stage) and updated
    /// each time [`advance_stage`] moves the flow forward. Used to populate
    /// [`StageHistoryEntry::entered_at`] when the stage is later exited.
    ///
    /// [`advance_stage`]: crate::engine::state::advance_stage
    pub stage_entered_at: DateTime<Utc>,

    /// The UTC instant at which this flow was first created.
    pub created_at: DateTime<Utc>,

    /// The UTC instant of the most recent mutation to this flow's state.
    pub updated_at: DateTime<Utc>,

    /// When `true`, this flow has been archived and will not appear in the
    /// default active flow listing.
    pub archived: bool,

    /// Ordered record of every stage the flow has entered, including the
    /// current stage.
    #[serde(default)]
    pub stage_history: Vec<StageHistoryEntry>,

    /// Ordered log of every step that has completed (pass or fail) within this
    /// flow.
    #[serde(default)]
    pub step_log: Vec<StepLogEntry>,
}

#[cfg(test)]
#[path = "flow.tests.rs"]
mod tests;
