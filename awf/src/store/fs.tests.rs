use assert_fs::TempDir;
use assert_fs::prelude::*;
use chrono::DateTime;

use super::archive_flow;
use super::artifact_exists;
use super::create_flow;
use super::create_workflow_template;
use super::list_flows;
use super::list_workflows;
use super::load_flow;
use super::load_prompt_file;
use super::load_workflow;
use super::put_artifact;
use super::read_artifact;
use super::save_flow;
use crate::model::flow::FlowState;
use crate::result::Error;

const ADD_OAUTH_STATE_TOML: &str = r#"
id = "add-oauth"
workflow = "feature"
workflow_version = 1
current_stage = "design"
stage_entered_at = "2026-04-04T10:00:00Z"
created_at = "2026-04-04T10:00:00Z"
updated_at = "2026-04-04T10:00:00Z"
archived = false
"#;

const FEATURE_TOML: &str = r#"
name = "feature"
version = 1
description = "Ship a new feature."

[commands]
test = "cargo test"

[[stages]]
name = "design"
description = "Produce a design doc."
produces = ["design.md"]
steps = [
    { name = "write-design", type = "agent", prompt = "design.md" },
]

[[stages]]
name = "implement"
requires = ["design.md"]
steps = [
    { name = "test", type = "command", command = "test" },
]
"#;

#[test]
fn list_workflows_returns_empty_for_empty_dir() {
    // Given: A temp `.awf/` directory with an empty `workflows/` subdirectory
    let awf_root = TempDir::new().unwrap();
    awf_root.child("workflows").create_dir_all().unwrap();

    // When: We list all workflows
    let result = list_workflows(awf_root.path());

    // Then: An empty list is returned
    assert_eq!(result.unwrap(), Vec::<String>::new());
}

#[test]
fn list_workflows_returns_sorted_stems() {
    // Given: A temp `.awf/` directory with three workflow files
    let awf_root = TempDir::new().unwrap();
    awf_root.child("workflows").create_dir_all().unwrap();
    awf_root.child("workflows/feature.toml").write_str("").unwrap();
    awf_root.child("workflows/hotfix.toml").write_str("").unwrap();
    awf_root.child("workflows/deploy.toml").write_str("").unwrap();

    // When: We list all workflows
    let result = list_workflows(awf_root.path());

    // Then: The names are returned sorted alphabetically, without the .toml
    // suffix
    assert_eq!(result.unwrap(), vec!["deploy", "feature", "hotfix"]);
}

#[test]
fn list_workflows_returns_empty_when_dir_missing() {
    // Given: A temp `.awf/` directory with no `workflows/` subdirectory
    let awf_root = TempDir::new().unwrap();

    // When: We list all workflows
    let result = list_workflows(awf_root.path());

    // Then: An empty list is returned rather than an error
    assert_eq!(result.unwrap(), Vec::<String>::new());
}

#[test]
fn loads_valid_workflow_from_fixture() {
    // Given: A temp `.awf/` directory with a valid `feature.toml` workflow file
    let awf_root = TempDir::new().unwrap();
    awf_root.child("workflows").create_dir_all().unwrap();
    awf_root.child("workflows/feature.toml").write_str(FEATURE_TOML).unwrap();

    // When: We load the "feature" workflow
    let result = load_workflow(awf_root.path(), "feature");

    // Then: The workflow is parsed correctly with the expected name and stages
    let workflow = result.unwrap();
    assert_eq!(workflow.name, "feature");
    assert_eq!(workflow.stages.len(), 2);
    assert_eq!(workflow.stages[0].name, "design");
    assert_eq!(workflow.stages[1].name, "implement");
}

#[test]
fn returns_workflow_not_found_for_missing_file() {
    // Given: A temp `.awf/` directory with no workflow files
    let awf_root = TempDir::new().unwrap();
    awf_root.child("workflows").create_dir_all().unwrap();

    // When: We attempt to load a workflow that does not exist
    let result = load_workflow(awf_root.path(), "nonexistent");

    // Then: WorkflowNotFound is returned with the correct name
    assert!(
        matches!(result, Err(Error::WorkflowNotFound { ref name }) if name == "nonexistent"),
        "expected WorkflowNotFound {{ name: \"nonexistent\" }}, got {result:?}",
    );
}

#[test]
fn returns_workflow_parse_failed_for_invalid_toml() {
    // Given: A temp `.awf/` directory with a malformed `broken.toml` file
    let awf_root = TempDir::new().unwrap();
    awf_root.child("workflows").create_dir_all().unwrap();
    awf_root
        .child("workflows/broken.toml")
        .write_str("this is not valid toml ][[[")
        .unwrap();

    // When: We attempt to load the malformed workflow
    let result = load_workflow(awf_root.path(), "broken");

    // Then: WorkflowParseFailed is returned with the correct name
    assert!(
        matches!(result, Err(Error::WorkflowParseFailed { ref name, .. }) if name == "broken"),
        "expected WorkflowParseFailed {{ name: \"broken\", .. }}, got {result:?}",
    );
}

#[test]
fn create_workflow_template_writes_file_with_correct_name() {
    // Given: A temp `.awf/` directory with no workflows yet
    let awf_root = TempDir::new().unwrap();

    // When: We create a workflow template named "my-workflow"
    let result = create_workflow_template(awf_root.path(), "my-workflow");

    // Then: The call succeeds and the file exists containing the workflow name
    result.unwrap();
    let toml_path = awf_root.path().join("workflows/my-workflow.toml");
    assert!(
        toml_path.exists(),
        "expected workflow file to exist at {toml_path:?}"
    );
    let contents = std::fs::read_to_string(&toml_path).unwrap();
    assert!(
        contents.contains(r#"name = "my-workflow""#),
        "expected file to contain workflow name, got:\n{contents}",
    );
}

#[test]
fn create_workflow_template_returns_already_exists_on_duplicate() {
    // Given: A temp `.awf/` directory where "my-workflow" has already been
    // created
    let awf_root = TempDir::new().unwrap();
    create_workflow_template(awf_root.path(), "my-workflow").unwrap();

    // When: We attempt to create the same workflow a second time
    let result = create_workflow_template(awf_root.path(), "my-workflow");

    // Then: WorkflowAlreadyExists is returned with the correct name
    assert!(
        matches!(result, Err(Error::WorkflowAlreadyExists { ref name }) if name == "my-workflow"),
        "expected WorkflowAlreadyExists {{ name: \"my-workflow\" }}, got {result:?}",
    );
}

#[test]
fn create_workflow_template_returns_invalid_id_for_name_with_space() {
    // Given: A temp `.awf/` directory
    let awf_root = TempDir::new().unwrap();

    // When: We attempt to create a workflow whose name contains a space
    let result = create_workflow_template(awf_root.path(), "has space");

    // Then: InvalidId is returned and no file is written
    assert!(
        matches!(result, Err(Error::InvalidId { ref id, .. }) if id == "has space"),
        "expected InvalidId {{ id: \"has space\", .. }}, got {result:?}",
    );
    let bad_path = awf_root.path().join("workflows/has space.toml");
    assert!(
        !bad_path.exists(),
        "expected no file to be created for invalid name"
    );
}

#[test]
fn loads_valid_flow_state_from_fixture() {
    // Given: A temp state directory with a valid `state.toml` for the
    // "add-oauth" flow
    let state_dir = TempDir::new().unwrap();
    state_dir
        .child("flows/active/add-oauth/state.toml")
        .write_str(ADD_OAUTH_STATE_TOML)
        .unwrap();

    // When: We load the "add-oauth" flow state
    let result = load_flow(state_dir.path(), "add-oauth");

    // Then: The flow state is parsed correctly with the expected fields
    let flow = result.unwrap();
    assert_eq!(flow.id, "add-oauth");
    assert_eq!(flow.workflow, "feature");
    assert_eq!(flow.current_stage, "design");
}

#[test]
fn returns_flow_not_found_for_missing_flow() {
    // Given: A temp state directory with no flows
    let state_dir = TempDir::new().unwrap();

    // When: We attempt to load a flow that does not exist
    let result = load_flow(state_dir.path(), "nonexistent");

    // Then: FlowNotFound is returned with the correct id
    assert!(
        matches!(result, Err(Error::FlowNotFound { ref id }) if id == "nonexistent"),
        "expected FlowNotFound {{ id: \"nonexistent\" }}, got {result:?}",
    );
}

#[test]
fn save_flow_round_trips_through_load_flow() {
    // Given: A temp state directory and a minimal FlowState in memory
    let state_dir = TempDir::new().unwrap();
    let original = FlowState {
        id: "test-flow".to_string(),
        workflow: "feature".to_string(),
        workflow_version: 1,
        current_stage: "design".to_string(),
        stage_entered_at: DateTime::parse_from_rfc3339("2026-04-04T10:00:00Z")
            .unwrap()
            .to_utc(),
        created_at: DateTime::parse_from_rfc3339("2026-04-04T10:00:00Z")
            .unwrap()
            .to_utc(),
        updated_at: DateTime::parse_from_rfc3339("2026-04-04T10:00:00Z")
            .unwrap()
            .to_utc(),
        archived: false,
        stage_history: vec![],
        step_log: vec![],
    };

    // When: We save the flow state and then load it back
    save_flow(state_dir.path(), &original).unwrap();
    let loaded = load_flow(state_dir.path(), "test-flow").unwrap();

    // Then: The loaded state is identical to the original
    assert_eq!(loaded, original);
}

#[test]
fn returns_flow_parse_failed_for_invalid_toml() {
    // Given: A temp state directory with a malformed `state.toml` for the
    // "broken-flow" flow
    let state_dir = TempDir::new().unwrap();
    state_dir
        .child("flows/active/broken-flow/state.toml")
        .write_str("this is not valid toml ][[[")
        .unwrap();

    // When: We attempt to load the malformed flow state
    let result = load_flow(state_dir.path(), "broken-flow");

    // Then: FlowParseFailed is returned with the correct id
    assert!(
        matches!(result, Err(Error::FlowParseFailed { ref id, .. }) if id == "broken-flow"),
        "expected FlowParseFailed {{ id: \"broken-flow\", .. }}, got {result:?}",
    );
}

#[test]
fn create_flow_creates_and_loads_initial_state() {
    // Given: A fresh temp state directory
    let state_dir = TempDir::new().unwrap();

    // When: We create the "add-oauth" flow under the "feature" workflow
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    // Then: The flow can be loaded back with the expected id, workflow, and
    // current stage
    let flow = load_flow(state_dir.path(), "add-oauth").unwrap();
    assert_eq!(flow.id, "add-oauth");
    assert_eq!(flow.workflow, "feature");
    assert_eq!(flow.current_stage, "design");
}

#[test]
fn create_flow_returns_already_exists_on_duplicate_active() {
    // Given: A temp state directory where "add-oauth" has already been created
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    // When: We attempt to create the same flow a second time
    let result =
        create_flow(state_dir.path(), "add-oauth", "feature", "design");

    // Then: FlowAlreadyExists is returned with the correct id
    assert!(
        matches!(result, Err(Error::FlowAlreadyExists { ref id }) if id == "add-oauth"),
        "expected FlowAlreadyExists {{ id: \"add-oauth\" }}, got {result:?}",
    );
}

#[test]
fn create_flow_returns_already_exists_when_archived_dir_exists() {
    // Given: A temp state directory where an archived directory for "add-oauth"
    // already exists
    let state_dir = TempDir::new().unwrap();
    state_dir.child("flows/archived/add-oauth").create_dir_all().unwrap();

    // When: We attempt to create the "add-oauth" flow
    let result =
        create_flow(state_dir.path(), "add-oauth", "feature", "design");

    // Then: FlowAlreadyExists is returned with the correct id
    assert!(
        matches!(result, Err(Error::FlowAlreadyExists { ref id }) if id == "add-oauth"),
        "expected FlowAlreadyExists {{ id: \"add-oauth\" }}, got {result:?}",
    );
}

#[test]
fn list_flows_returns_active_flows_sorted_by_id() {
    // Given: A temp state directory with two active flows
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "fix-bug", "hotfix", "patch").unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    // When: We list flows with include_archived = false
    let summaries = list_flows(state_dir.path(), false).unwrap();

    // Then: Both active flows are returned, sorted alphabetically by id
    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].id, "add-oauth");
    assert_eq!(summaries[0].workflow, "feature");
    assert_eq!(summaries[0].current_stage, "design");
    assert!(!summaries[0].archived);
    assert_eq!(summaries[1].id, "fix-bug");
    assert_eq!(summaries[1].workflow, "hotfix");
    assert_eq!(summaries[1].current_stage, "patch");
    assert!(!summaries[1].archived);
}

#[test]
fn list_flows_includes_archived_when_requested() {
    // Given: A temp state directory with one active flow and one archived flow
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();
    create_flow(state_dir.path(), "fix-bug", "hotfix", "patch").unwrap();

    // Move "fix-bug" to the archived directory to simulate archiving
    let active_dir = state_dir.path().join("flows/active/fix-bug");
    let archived_parent = state_dir.path().join("flows/archived");
    std::fs::create_dir_all(&archived_parent).unwrap();
    std::fs::rename(&active_dir, archived_parent.join("fix-bug")).unwrap();

    // When: We list flows with include_archived = true
    let all_summaries = list_flows(state_dir.path(), true).unwrap();

    // Then: Both flows are returned, sorted by id; archived flow is flagged
    assert_eq!(all_summaries.len(), 2);
    assert_eq!(all_summaries[0].id, "add-oauth");
    assert!(!all_summaries[0].archived);
    assert_eq!(all_summaries[1].id, "fix-bug");
    assert!(all_summaries[1].archived);

    // When: We list flows with include_archived = false
    let active_only = list_flows(state_dir.path(), false).unwrap();

    // Then: Only the active flow is returned
    assert_eq!(active_only.len(), 1);
    assert_eq!(active_only[0].id, "add-oauth");
}

// ---------------------------------------------------------------------------
// S11: put_artifact
// ---------------------------------------------------------------------------

#[test]
fn put_artifact_copies_file_into_artifact_dir() {
    // Given: An active "add-oauth" flow and a source file to store
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    let src = TempDir::new().unwrap();
    src.child("design.md").write_str("# Design\nHello.").unwrap();
    let src_path = src.path().join("design.md");

    // When: We store the source file as the "design.md" artifact
    put_artifact(state_dir.path(), "add-oauth", "design.md", &src_path)
        .unwrap();

    // Then: The file exists at the expected location with the original content
    let artifact_path =
        state_dir.path().join("flows/active/add-oauth/artifacts/design.md");
    assert!(
        artifact_path.exists(),
        "artifact file should exist at {artifact_path:?}"
    );
    let contents = std::fs::read_to_string(&artifact_path).unwrap();
    assert_eq!(contents, "# Design\nHello.");
}

// ---------------------------------------------------------------------------
// S12: artifact_exists
// ---------------------------------------------------------------------------

#[test]
fn artifact_exists_returns_true_when_artifact_is_present() {
    // Given: An active "add-oauth" flow with a stored "design.md" artifact
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    let src = TempDir::new().unwrap();
    src.child("design.md").write_str("content").unwrap();
    put_artifact(
        state_dir.path(),
        "add-oauth",
        "design.md",
        &src.path().join("design.md"),
    )
    .unwrap();

    // When / Then: artifact_exists returns true for the stored artifact
    assert!(artifact_exists(state_dir.path(), "add-oauth", "design.md"));
}

#[test]
fn artifact_exists_returns_false_when_artifact_is_absent() {
    // Given: An active "add-oauth" flow with no artifacts
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    // When / Then: artifact_exists returns false for a name that was never
    // stored
    assert!(!artifact_exists(state_dir.path(), "add-oauth", "missing.md"));
}

// ---------------------------------------------------------------------------
// S13: read_artifact
// ---------------------------------------------------------------------------

#[test]
fn read_artifact_returns_content_of_stored_artifact() {
    // Given: An active "add-oauth" flow with a "design.md" artifact containing
    // known text
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    let src = TempDir::new().unwrap();
    src.child("design.md").write_str("# My Design\nDetails here.").unwrap();
    put_artifact(
        state_dir.path(),
        "add-oauth",
        "design.md",
        &src.path().join("design.md"),
    )
    .unwrap();

    // When: We read the artifact back
    let contents =
        read_artifact(state_dir.path(), "add-oauth", "design.md").unwrap();

    // Then: The returned string matches the original file content
    assert_eq!(contents, "# My Design\nDetails here.");
}

#[test]
fn read_artifact_returns_io_error_for_missing_artifact() {
    // Given: An active "add-oauth" flow with no artifacts
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    // When: We attempt to read an artifact that was never stored
    let result = read_artifact(state_dir.path(), "add-oauth", "missing.md");

    // Then: An Io error is returned (file not found)
    assert!(
        matches!(result, Err(Error::Io(_))),
        "expected Err(Error::Io(_)), got {result:?}",
    );
}

// ---------------------------------------------------------------------------
// S14: archive_flow
// ---------------------------------------------------------------------------

#[test]
fn archive_flow_moves_flow_to_archived_dir_and_sets_archived_flag() {
    // Given: An active "add-oauth" flow
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();

    // When: We archive the flow
    archive_flow(state_dir.path(), "add-oauth").unwrap();

    // Then: The active directory no longer exists
    let active_dir = state_dir.path().join("flows/active/add-oauth");
    assert!(
        !active_dir.exists(),
        "active directory should be gone after archiving"
    );

    // Then: The archived state.toml exists and has archived = true
    let archived_state_path =
        state_dir.path().join("flows/archived/add-oauth/state.toml");
    assert!(
        archived_state_path.exists(),
        "archived state.toml should exist at {archived_state_path:?}"
    );
    let contents = std::fs::read_to_string(&archived_state_path).unwrap();
    assert!(
        contents.contains("archived = true"),
        "archived state.toml should contain `archived = true`, got:\n{contents}",
    );
}

#[test]
fn archive_flow_makes_load_flow_return_not_found() {
    // Given: An active "add-oauth" flow that we then archive
    let state_dir = TempDir::new().unwrap();
    create_flow(state_dir.path(), "add-oauth", "feature", "design").unwrap();
    archive_flow(state_dir.path(), "add-oauth").unwrap();

    // When: We try to load the flow via the normal active-only path
    let result = load_flow(state_dir.path(), "add-oauth");

    // Then: FlowNotFound is returned because load_flow only checks active/
    assert!(
        matches!(result, Err(Error::FlowNotFound { ref id }) if id == "add-oauth"),
        "expected FlowNotFound {{ id: \"add-oauth\" }}, got {result:?}",
    );
}

#[test]
fn archive_flow_returns_flow_not_found_for_nonexistent_flow() {
    // Given: A state directory with no flows
    let state_dir = TempDir::new().unwrap();

    // When: We attempt to archive a flow that does not exist
    let result = archive_flow(state_dir.path(), "ghost-flow");

    // Then: FlowNotFound is returned with the correct id
    assert!(
        matches!(result, Err(Error::FlowNotFound { ref id }) if id == "ghost-flow"),
        "expected FlowNotFound {{ id: \"ghost-flow\" }}, got {result:?}",
    );
}

// ---------------------------------------------------------------------------
// S15: load_prompt_file
// ---------------------------------------------------------------------------

#[test]
fn load_prompt_file_returns_content_of_workflow_prompt() {
    // Given: An awf root directory with a "feature/design.md" prompt file
    let awf_root = TempDir::new().unwrap();
    awf_root
        .child("workflows/feature/design.md")
        .write_str("# Design Prompt\nDo the thing.")
        .unwrap();

    // When: We load the prompt file
    let contents =
        load_prompt_file(awf_root.path(), "feature", "design.md").unwrap();

    // Then: The full file content is returned
    assert_eq!(contents, "# Design Prompt\nDo the thing.");
}

#[test]
fn load_prompt_file_returns_io_error_for_missing_file() {
    // Given: An awf root directory with no prompt files under "feature"
    let awf_root = TempDir::new().unwrap();
    awf_root.child("workflows/feature").create_dir_all().unwrap();

    // When: We attempt to load a prompt file that does not exist
    let result = load_prompt_file(awf_root.path(), "feature", "missing.md");

    // Then: An Io error is returned (file not found)
    assert!(
        matches!(result, Err(Error::Io(_))),
        "expected Err(Error::Io(_)), got {result:?}",
    );
}
