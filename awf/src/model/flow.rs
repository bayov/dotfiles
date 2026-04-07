//! Flow state types: [`FlowState`] and [`StageHistoryEntry`].
//!
//! A flow is a running instance of a workflow. Its state is persisted as a
//! TOML file in `.awf/flows/` and tracks which stage the flow is currently
//! in and the history of completed stage events.
//!
//! Flow state files are loaded and written verbatim — there is no validation
//! step analogous to [`super::workflow::Workflow::validate`] because the
//! runtime is responsible for maintaining invariants during mutations.

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

/// The outcome of a completed stage event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// The stage finished successfully.
    Completed,

    /// The stage finished with a failure (non-zero exit code for commands;
    /// agent returned an error for agent stages).
    Failed,

    /// The stage was force-advanced past gate checks despite a failure.
    ForceCompleted,

    /// The stage was skipped entirely via `next-stage --skip`.
    Skipped,
}

/// A record of a single completed stage event within a flow.
///
/// Appended to [`FlowState::stage_history`] whenever a stage run finishes
/// (pass or fail) or whenever the flow transitions out of a stage via force or
/// skip. Multiple entries for the same stage name are possible when a stage
/// is run, fails, and is then force-advanced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageHistoryEntry {
    /// The name of the stage (e.g. `"design"`).
    pub stage: String,

    /// The UTC instant at which this event occurred.
    pub time: DateTime<Utc>,

    /// The outcome of this event.
    pub status: StageStatus,

    /// Human-readable reason provided when the stage was force-advanced.
    /// Only present when `status` is [`StageStatus::ForceCompleted`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub force_reason: Option<String>,

    /// Wall-clock duration of the command execution in milliseconds.
    /// Only present for command stages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
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
    /// each time a stage transition moves the flow forward. Used to populate
    /// [`StageHistoryEntry::time`] when a run or transition is recorded.
    pub stage_started_at: DateTime<Utc>,

    /// The UTC instant at which this flow was first created.
    pub created_at: DateTime<Utc>,

    /// The UTC instant of the most recent mutation to this flow's state.
    pub updated_at: DateTime<Utc>,

    /// When `true`, this flow has been archived and will not appear in the
    /// default active flow listing.
    pub archived: bool,

    /// Ordered log of every completed stage event in this flow.
    ///
    /// Entries are appended when a stage run finishes (pass or fail) or when
    /// the flow transitions out of a stage via force or skip. The current
    /// stage is never represented here until it produces an event.
    #[serde(default)]
    pub stage_history: Vec<StageHistoryEntry>,
}

#[cfg(test)]
#[path = "flow.tests.rs"]
mod tests;
