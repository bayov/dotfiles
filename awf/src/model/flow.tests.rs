use chrono::DateTime;
use chrono::Utc;

use super::FlowState;
use super::StageHistoryEntry;
use super::StageStatus;

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
// TOML round-trip: full flow state
// ---------------------------------------------------------------------------

#[test]
fn full_flow_state_round_trip() {
    // Given: a FlowState built in memory with a variety of history entries
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
                time: ts("2026-04-04T12:00:00Z"),
                status: StageStatus::Completed,
                force_reason: None,
                duration_ms: None,
            },
            StageHistoryEntry {
                stage: "lint".to_owned(),
                time: ts("2026-04-04T14:00:00Z"),
                status: StageStatus::Failed,
                force_reason: None,
                duration_ms: Some(800),
            },
            StageHistoryEntry {
                stage: "lint".to_owned(),
                time: ts("2026-04-04T14:01:00Z"),
                status: StageStatus::ForceCompleted,
                force_reason: Some("known flake".to_owned()),
                duration_ms: None,
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
// TOML parse: canonical format
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
        time = "2026-04-04T12:00:00Z"
        status = "completed"

        [[stage_history]]
        stage = "lint"
        time = "2026-04-04T14:00:00Z"
        status = "failed"
        duration_ms = 800

        [[stage_history]]
        stage = "lint"
        time = "2026-04-04T14:01:00Z"
        status = "force_completed"
        force_reason = "known flake"
    "#;

    // When: we parse the TOML
    let flow = parse(toml_input);

    // Then: top-level fields match
    assert_eq!(flow.id, "add-oauth");
    assert_eq!(flow.current_stage, "format");
    assert_eq!(flow.created_at, ts("2026-04-04T10:00:00Z"));

    // Then: stage history has three entries
    assert_eq!(flow.stage_history.len(), 3);

    let e0 = &flow.stage_history[0];
    assert_eq!(e0.stage, "design");
    assert_eq!(e0.time, ts("2026-04-04T12:00:00Z"));
    assert_eq!(e0.status, StageStatus::Completed);
    assert!(e0.force_reason.is_none());
    assert!(e0.duration_ms.is_none());

    let e1 = &flow.stage_history[1];
    assert_eq!(e1.stage, "lint");
    assert_eq!(e1.status, StageStatus::Failed);
    assert_eq!(e1.duration_ms, Some(800));

    let e2 = &flow.stage_history[2];
    assert_eq!(e2.stage, "lint");
    assert_eq!(e2.status, StageStatus::ForceCompleted);
    assert_eq!(e2.force_reason.as_deref(), Some("known flake"));
}

// ---------------------------------------------------------------------------
// StageStatus serialization
// ---------------------------------------------------------------------------

#[test]
fn stage_status_serializes_as_snake_case() {
    // Given: a flow with one history entry for each status variant
    let statuses = [
        (StageStatus::Completed, "completed"),
        (StageStatus::Failed, "failed"),
        (StageStatus::ForceCompleted, "force_completed"),
        (StageStatus::Skipped, "skipped"),
    ];

    for (status, expected) in statuses {
        let flow = FlowState {
            id: "x".to_owned(),
            workflow: "w".to_owned(),
            workflow_version: 1,
            current_stage: "s".to_owned(),
            stage_started_at: ts("2026-04-04T10:00:00Z"),
            created_at: ts("2026-04-04T10:00:00Z"),
            updated_at: ts("2026-04-04T10:00:00Z"),
            archived: false,
            stage_history: vec![StageHistoryEntry {
                stage: "s".to_owned(),
                time: ts("2026-04-04T10:00:00Z"),
                status,
                force_reason: None,
                duration_ms: None,
            }],
        };

        let serialized =
            toml::to_string(&flow).expect("serialization should succeed");

        assert!(
            serialized.contains(&format!(r#"status = "{expected}""#)),
            "expected 'status = \"{expected}\"' in:\n{serialized}"
        );
    }
}

// ---------------------------------------------------------------------------
// Optional fields: force_reason and duration_ms omitted when None
// ---------------------------------------------------------------------------

#[test]
fn optional_fields_omitted_when_none() {
    // Given: a completed entry with no force_reason or duration_ms
    let entry = StageHistoryEntry {
        stage: "design".to_owned(),
        time: ts("2026-04-04T12:00:00Z"),
        status: StageStatus::Completed,
        force_reason: None,
        duration_ms: None,
    };

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
    };
    let serialized =
        toml::to_string(&flow).expect("serialization should succeed");

    // Then: optional fields are absent
    assert!(
        !serialized.contains("force_reason"),
        "force_reason should be omitted when None"
    );
    assert!(
        !serialized.contains("duration_ms"),
        "duration_ms should be omitted when None"
    );

    // Then: the round-trip preserves the None values
    let roundtripped: FlowState =
        toml::from_str(&serialized).expect("deserialization should succeed");
    assert_eq!(roundtripped.stage_history[0].force_reason, None);
    assert_eq!(roundtripped.stage_history[0].duration_ms, None);
}
