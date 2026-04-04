use assert_fs::TempDir;
use assert_fs::prelude::*;

use super::load_workflow;
use crate::result::Error;

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
