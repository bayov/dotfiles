use chrono::DateTime;
use chrono::Utc;

use super::FlowState;
use super::RunStatus;
use super::StageHistoryEntry;
use super::StageLogEntry;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse an RFC 3339 timestamp string into a [`DateTime<Utc>`], panicking on
/// failure.
fn ts(s: &str) -> DateTime<Utc> { s.parse().expect("timestamp should parse") }

/// Deserialize a TOML string into a [`FlowState`], panicking on parse failure.
fn parse(toml: &str) -> FlowState {
    toml::from_str(toml).expect("TOML should parse without error")
}

// ---------------------------------------------------------------------------
// TOML round-trip: full flow state with both stage kinds
// ---------------------------------------------------------------------------

#[test]
fn full_flow_state_round_trip() {
    // Given: a FlowState built in memory with both agent and command stage log
    // entries and stage history with and without completed_at
    let original = FlowState {
        id: "add-oauth".to_owned(),
        workflow: "feature".to_owned(),
        workflow_version: 1,
        current_stage: "format".to_owned(),
        stage_started_at: ts("2026-04-04T12:00:00Z"),
        created_at: ts("2026-04-04T10:00:00Z"),
        updated_at: ts("2026-04-04T14:32:00Z"),
        archived: false,
        stage_history: vec![
            StageHistoryEntry {
                stage: "design".to_owned(),
                started_at: ts("2026-04-04T10:00:00Z"),
                completed_at: Some(ts("2026-04-04T12:00:00Z")),
                force_completed: false,
                force_reason: None,
                skipped: false,
            },
            StageHistoryEntry {
                stage: "implement".to_owned(),
                started_at: ts("2026-04-04T12:00:00Z"),
                completed_at: None,
                force_completed: false,
                force_reason: None,
                skipped: false,
            },
        ],
        stage_log: vec![
            // Agent stage
            StageLogEntry {
                stage: "design".to_owned(),
                stage_kind: "agent".to_owned(),
                status: RunStatus::Pass,
                exit_code: None,
                started_at: None,
                duration_ms: None,
                completed_at: Some(ts("2026-04-04T13:00:00Z")),
            },
            // Passing command stage
            StageLogEntry {
                stage: "format".to_owned(),
                stage_kind: "command".to_owned(),
                status: RunStatus::Pass,
                exit_code: Some(0),
                started_at: Some(ts("2026-04-04T14:30:00Z")),
                duration_ms: Some(1200),
                completed_at: None,
            },
            // Failing command stage
            StageLogEntry {
                stage: "lint".to_owned(),
                stage_kind: "command".to_owned(),
                status: RunStatus::Fail,
                exit_code: Some(1),
                started_at: Some(ts("2026-04-04T14:31:00Z")),
                duration_ms: Some(8500),
                completed_at: None,
            },
        ],
    };

    // When: we serialize to TOML and deserialize back
    let serialized =
        toml::to_string(&original).expect("serialization should succeed");
    let roundtripped: FlowState =
        toml::from_str(&serialized).expect("deserialization should succeed");

    // Then: the roundtripped value is identical to the original
    assert_eq!(original, roundtripped);
}

// ---------------------------------------------------------------------------
// TOML parse: canonical format from design doc
// ---------------------------------------------------------------------------

#[test]
fn parse_canonical_toml_format() {
    // Given: the canonical TOML format
    let toml_input = r#"
        id = "add-oauth"
        workflow = "feature"
        workflow_version = 1
        current_stage = "format"
        stage_started_at = "2026-04-04T12:00:00Z"
        created_at = "2026-04-04T10:00:00Z"
        updated_at = "2026-04-04T14:32:00Z"
        archived = false

        [[stage_history]]
        stage = "design"
        started_at = "2026-04-04T10:00:00Z"
        completed_at = "2026-04-04T12:00:00Z"

        [[stage_log]]
        stage = "implement"
        kind = "agent"
        status = "pass"
        completed_at = "2026-04-04T13:00:00Z"

        [[stage_log]]
        stage = "format"
        kind = "command"
        status = "pass"
        exit_code = 0
        started_at = "2026-04-04T14:30:00Z"
        duration_ms = 1200

        [[stage_log]]
        stage = "lint"
        kind = "command"
        status = "fail"
        exit_code = 1
        started_at = "2026-04-04T14:31:00Z"
        duration_ms = 8500
    "#;

    // When: we parse the TOML
    let flow = parse(toml_input);

    // Then: top-level fields match
    assert_eq!(flow.id, "add-oauth");
    assert_eq!(flow.workflow, "feature");
    assert_eq!(flow.workflow_version, 1);
    assert_eq!(flow.current_stage, "format");
    assert_eq!(flow.created_at, ts("2026-04-04T10:00:00Z"));
    assert_eq!(flow.updated_at, ts("2026-04-04T14:32:00Z"));
    assert!(!flow.archived);

    // Then: stage history
    assert_eq!(flow.stage_history.len(), 1);
    let history_entry = &flow.stage_history[0];
    assert_eq!(history_entry.stage, "design");
    assert_eq!(history_entry.started_at, ts("2026-04-04T10:00:00Z"));
    assert_eq!(history_entry.completed_at, Some(ts("2026-04-04T12:00:00Z")));

    // Then: stage log has three entries
    assert_eq!(flow.stage_log.len(), 3);

    // Then: agent stage entry
    let agent_entry = &flow.stage_log[0];
    assert_eq!(agent_entry.stage, "implement");
    assert_eq!(agent_entry.stage_kind, "agent");
    assert_eq!(agent_entry.status, RunStatus::Pass);
    assert_eq!(agent_entry.completed_at, Some(ts("2026-04-04T13:00:00Z")));
    assert!(agent_entry.exit_code.is_none());
    assert!(agent_entry.started_at.is_none());
    assert!(agent_entry.duration_ms.is_none());

    // Then: passing command stage entry
    let format_entry = &flow.stage_log[1];
    assert_eq!(format_entry.stage, "format");
    assert_eq!(format_entry.stage_kind, "command");
    assert_eq!(format_entry.status, RunStatus::Pass);
    assert_eq!(format_entry.exit_code, Some(0));
    assert_eq!(format_entry.started_at, Some(ts("2026-04-04T14:30:00Z")));
    assert_eq!(format_entry.duration_ms, Some(1200));
    assert!(format_entry.completed_at.is_none());

    // Then: failing command stage entry
    let test_entry = &flow.stage_log[2];
    assert_eq!(test_entry.stage, "lint");
    assert_eq!(test_entry.stage_kind, "command");
    assert_eq!(test_entry.status, RunStatus::Fail);
    assert_eq!(test_entry.exit_code, Some(1));
    assert_eq!(test_entry.started_at, Some(ts("2026-04-04T14:31:00Z")));
    assert_eq!(test_entry.duration_ms, Some(8500));
    assert!(test_entry.completed_at.is_none());
}

// ---------------------------------------------------------------------------
// RunStatus serialization
// ---------------------------------------------------------------------------

#[test]
fn run_status_serializes_as_lowercase() {
    // Given: both RunStatus variants
    // When: we serialize them as part of a minimal FlowState
    let mut flow = FlowState {
        id: "x".to_owned(),
        workflow: "w".to_owned(),
        workflow_version: 1,
        current_stage: "s".to_owned(),
        stage_started_at: ts("2026-04-04T10:00:00Z"),
        created_at: ts("2026-04-04T10:00:00Z"),
        updated_at: ts("2026-04-04T10:00:00Z"),
        archived: false,
        stage_history: vec![],
        stage_log: vec![StageLogEntry {
            stage: "s".to_owned(),
            stage_kind: "command".to_owned(),
            status: RunStatus::Pass,
            exit_code: None,
            started_at: None,
            duration_ms: None,
            completed_at: None,
        }],
    };

    let serialized =
        toml::to_string(&flow).expect("serialization should succeed");

    // Then: status appears as "pass" (lowercase)
    assert!(
        serialized.contains(r#"status = "pass""#),
        "expected 'status = \"pass\"' in:\n{serialized}"
    );

    // When: we change the status to Fail and re-serialize
    flow.stage_log[0].status = RunStatus::Fail;
    let serialized =
        toml::to_string(&flow).expect("serialization should succeed");

    // Then: status appears as "fail" (lowercase)
    assert!(
        serialized.contains(r#"status = "fail""#),
        "expected 'status = \"fail\"' in:\n{serialized}"
    );
}

// ---------------------------------------------------------------------------
// Stage history: active stage has no completed_at
// ---------------------------------------------------------------------------

#[test]
fn stage_history_entry_without_completed_at_omits_field() {
    // Given: a stage history entry with no completed_at (stage still active)
    let entry = StageHistoryEntry {
        stage: "implement".to_owned(),
        started_at: ts("2026-04-04T12:00:00Z"),
        completed_at: None,
        force_completed: false,
        force_reason: None,
        skipped: false,
    };

    // When: we embed it in a flow state and serialize
    let flow = FlowState {
        id: "x".to_owned(),
        workflow: "w".to_owned(),
        workflow_version: 1,
        current_stage: "implement".to_owned(),
        stage_started_at: ts("2026-04-04T12:00:00Z"),
        created_at: ts("2026-04-04T10:00:00Z"),
        updated_at: ts("2026-04-04T12:00:00Z"),
        archived: false,
        stage_history: vec![entry],
        stage_log: vec![],
    };
    let serialized =
        toml::to_string(&flow).expect("serialization should succeed");

    // Then: completed_at is absent from the serialized output
    assert!(
        !serialized.contains("completed_at"),
        "completed_at should be omitted when None, but found in:\n{serialized}"
    );

    // Then: the round-trip preserves the None value
    let roundtripped: FlowState =
        toml::from_str(&serialized).expect("deserialization should succeed");
    assert_eq!(roundtripped.stage_history[0].completed_at, None);
}
