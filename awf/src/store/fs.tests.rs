use assert_fs::TempDir;
use assert_fs::prelude::*;
use chrono::DateTime;

use super::create_workflow_template;
use super::list_workflows;
use super::load_flow;
use super::load_workflow;
use super::save_flow;
use crate::model::flow::FlowState;
use crate::result::Error;

const ADD_OAUTH_STATE_TOML: &str = r#"
id = "add-oauth"
workflow = "feature"
workflow_version = 1
current_stage = "design"
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
