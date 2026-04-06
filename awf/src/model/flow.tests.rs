use chrono::DateTime;
use chrono::Utc;

use super::FlowState;
use super::StageHistoryEntry;
use super::StepLogEntry;
use super::StepStatus;

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
// TOML round-trip: full flow state with both step kinds
// ---------------------------------------------------------------------------

#[test]
fn full_flow_state_round_trip() {
    // Given: a FlowState built in memory with both agent and command step log
    // entries and stage history with and without completed_at
    let original = FlowState {
        id: "add-oauth".to_owned(),
        workflow: "feature".to_owned(),
        workflow_version: 1,
        current_stage: "implement".to_owned(),
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
            },
            StageHistoryEntry {
                stage: "implement".to_owned(),
                started_at: ts("2026-04-04T12:00:00Z"),
                completed_at: None,
                force_completed: false,
                force_reason: None,
            },
        ],
        step_log: vec![
            // Agent step
            StepLogEntry {
                step: "write-design".to_owned(),
                stage: "design".to_owned(),
                step_type: "agent".to_owned(),
                status: StepStatus::Pass,
                exit_code: None,
                started_at: None,
                duration_ms: None,
                completed_at: Some(ts("2026-04-04T13:00:00Z")),
            },
            // Passing command step
            StepLogEntry {
                step: "format".to_owned(),
                stage: "implement".to_owned(),
                step_type: "command".to_owned(),
                status: StepStatus::Pass,
                exit_code: Some(0),
                started_at: Some(ts("2026-04-04T14:30:00Z")),
                duration_ms: Some(1200),
                completed_at: None,
            },
            // Failing command step
            StepLogEntry {
                step: "test".to_owned(),
                stage: "implement".to_owned(),
                step_type: "command".to_owned(),
                status: StepStatus::Fail,
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
    // Given: the canonical TOML format from the design doc
    let toml_input = r#"
        id = "add-oauth"
        workflow = "feature"
        workflow_version = 1
        current_stage = "implement"
        stage_started_at = "2026-04-04T12:00:00Z"
        created_at = "2026-04-04T10:00:00Z"
        updated_at = "2026-04-04T14:32:00Z"
        archived = false

        [[stage_history]]
        stage = "design"
        started_at = "2026-04-04T10:00:00Z"
        completed_at = "2026-04-04T12:00:00Z"

        [[step_log]]
        step = "write-code"
        stage = "implement"
        type = "agent"
        status = "pass"
        completed_at = "2026-04-04T13:00:00Z"

        [[step_log]]
        step = "format"
        stage = "implement"
        type = "command"
        status = "pass"
        exit_code = 0
        started_at = "2026-04-04T14:30:00Z"
        duration_ms = 1200

        [[step_log]]
        step = "test"
        stage = "implement"
        type = "command"
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
    assert_eq!(flow.current_stage, "implement");
    assert_eq!(flow.created_at, ts("2026-04-04T10:00:00Z"));
    assert_eq!(flow.updated_at, ts("2026-04-04T14:32:00Z"));
    assert!(!flow.archived);

    // Then: stage history
    assert_eq!(flow.stage_history.len(), 1);
    let history_entry = &flow.stage_history[0];
    assert_eq!(history_entry.stage, "design");
    assert_eq!(history_entry.started_at, ts("2026-04-04T10:00:00Z"));
    assert_eq!(history_entry.completed_at, Some(ts("2026-04-04T12:00:00Z")));

    // Then: step log has three entries
    assert_eq!(flow.step_log.len(), 3);

    // Then: agent step
    let agent_entry = &flow.step_log[0];
    assert_eq!(agent_entry.step, "write-code");
    assert_eq!(agent_entry.stage, "implement");
    assert_eq!(agent_entry.step_type, "agent");
    assert_eq!(agent_entry.status, StepStatus::Pass);
    assert_eq!(agent_entry.completed_at, Some(ts("2026-04-04T13:00:00Z")));
    assert!(agent_entry.exit_code.is_none());
    assert!(agent_entry.started_at.is_none());
    assert!(agent_entry.duration_ms.is_none());

    // Then: passing command step
    let format_entry = &flow.step_log[1];
    assert_eq!(format_entry.step, "format");
    assert_eq!(format_entry.step_type, "command");
    assert_eq!(format_entry.status, StepStatus::Pass);
    assert_eq!(format_entry.exit_code, Some(0));
    assert_eq!(format_entry.started_at, Some(ts("2026-04-04T14:30:00Z")));
    assert_eq!(format_entry.duration_ms, Some(1200));
    assert!(format_entry.completed_at.is_none());

    // Then: failing command step
    let test_entry = &flow.step_log[2];
    assert_eq!(test_entry.step, "test");
    assert_eq!(test_entry.step_type, "command");
    assert_eq!(test_entry.status, StepStatus::Fail);
    assert_eq!(test_entry.exit_code, Some(1));
    assert_eq!(test_entry.started_at, Some(ts("2026-04-04T14:31:00Z")));
    assert_eq!(test_entry.duration_ms, Some(8500));
    assert!(test_entry.completed_at.is_none());
}

// ---------------------------------------------------------------------------
// StepStatus serialization
// ---------------------------------------------------------------------------

#[test]
fn step_status_serializes_as_lowercase() {
    // Given: both StepStatus variants
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
        step_log: vec![StepLogEntry {
            step: "a".to_owned(),
            stage: "s".to_owned(),
            step_type: "command".to_owned(),
            status: StepStatus::Pass,
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
    flow.step_log[0].status = StepStatus::Fail;
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
        step_log: vec![],
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
