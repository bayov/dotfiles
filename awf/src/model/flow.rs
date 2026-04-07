//! Flow state types: [`FlowState`], [`StageHistoryEntry`], [`StageLogEntry`],
//! and [`RunStatus`].
//!
//! A flow is a running instance of a workflow. Its state is persisted as a
//! TOML file in `.awf/flows/` and tracks which stage the flow is currently
//! in, the history of stage transitions, and a log of completed stage
//! executions.
//!
//! Flow state files are loaded and written verbatim — there is no validation
//! step analogous to [`super::workflow::Workflow::validate`] because the
//! runtime is responsible for maintaining invariants during mutations.

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// The outcome of a completed stage execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    /// The stage finished successfully (exit code 0 for commands; agent
    /// completed without error for agent stages).
    Pass,

    /// The stage finished with a failure (non-zero exit code for commands;
    /// agent returned an error for agent stages).
    Fail,
}

/// A record of a single stage transition within a flow.
///
/// Appended to [`FlowState::stage_history`] whenever the flow advances from
/// one stage to the next. The `completed_at` field is `None` while the stage
/// is still active and is filled in when the flow leaves the stage.
/// When the stage was force-advanced (skipping gate checks),
/// `force_completed` is set to `true`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageHistoryEntry {
    /// The name of the stage (e.g. `"design"`).
    pub stage: String,

    /// The UTC instant at which the flow entered this stage.
    pub started_at: DateTime<Utc>,

    /// The UTC instant at which the flow left this stage, or `None` if the
    /// stage is still active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,

    /// `true` when the stage was force-advanced past gate checks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub force_completed: bool,

    /// Human-readable reason why the stage was force-advanced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_reason: Option<String>,

    /// `true` when the stage was skipped entirely via `next-stage --skip`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub skipped: bool,
}

/// A single completed stage execution recorded in a flow's stage log.
///
/// Command stages and agent stages share this struct; fields that do not apply
/// to a given stage kind are stored as `None` and omitted during serialization.
///
/// **Command-specific fields** (present when `stage_kind` is `"command"`):
/// `exit_code`, `started_at`, `duration_ms`. `completed_at` is also set for
/// command stages, computed as `started_at + duration_ms`.
///
/// **Agent-specific fields** (present when `stage_kind` is `"agent"`):
/// `completed_at`.
///
/// The `stage_kind` field (`kind` in TOML) mirrors the `kind` tag used in
/// workflow stage definitions and allows tooling to distinguish the two kinds
/// of entries without inspecting which optional fields are populated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageLogEntry {
    /// The name of the stage (e.g. `"implement"`).
    pub stage: String,

    /// The kind of stage: `"command"` or `"agent"` in TOML.
    #[serde(rename = "kind")]
    pub stage_kind: String,

    /// Whether the stage run passed or failed.
    pub status: RunStatus,

    // -----------------------------------------------------------------------
    // Command-specific fields
    // -----------------------------------------------------------------------
    /// The process exit code (command stages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,

    /// The UTC instant at which the command process was started (command
    /// stages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,

    /// Wall-clock duration of the command execution in milliseconds (command
    /// stages only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    // -----------------------------------------------------------------------
    // Agent-specific fields
    // -----------------------------------------------------------------------
    /// The UTC instant at which the agent stage completed (agent stages only).
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
    /// [`StageHistoryEntry::started_at`] when the stage is later completed.
    ///
    /// [`advance_stage`]: crate::engine::state::advance_stage
    pub stage_started_at: DateTime<Utc>,

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

    /// Ordered log of every stage execution that has completed (pass or fail)
    /// within this flow.
    #[serde(default)]
    pub stage_log: Vec<StageLogEntry>,
}

#[cfg(test)]
#[path = "flow.tests.rs"]
mod tests;
