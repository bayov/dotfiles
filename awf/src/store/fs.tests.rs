use assert_fs::TempDir;
use assert_fs::prelude::*;
use chrono::DateTime;

use super::archive_flow;
use super::artifact_exists;
use super::create_flow;
use super::create_workflow_template;
use super::find_workflow_dir;
use super::list_workflows;
use super::load_flow;
use super::load_prompt_file;
use super::load_workflow;
use super::put_artifact;
use super::read_artifact;
use super::save_flow;
use super::scan_flows;
use crate::config::WorkflowSearchEntry;
use crate::model::flow::FlowState;
use crate::result::Error;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_entry(
    dir: impl Into<std::path::PathBuf>,
    scope: &str,
) -> WorkflowSearchEntry {
    WorkflowSearchEntry { dir: dir.into(), scope: scope.to_string() }
}

const FEATURE_WORKFLOW_TOML: &str = r#"
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

// ---------------------------------------------------------------------------
// list_workflows
// ---------------------------------------------------------------------------

#[test]
fn list_workflows_returns_empty_when_entries_have_no_workflows() {
    // Given: an empty agents/workflows dir
    let root = TempDir::new().unwrap();
    root.child("workflows").create_dir_all().unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: we list workflows
    let result = list_workflows(&entries).unwrap();

    // Then: empty list
    assert!(result.is_empty());
}

#[test]
fn list_workflows_finds_dirs_with_workflow_toml() {
    // Given: two workflow subdirs with WORKFLOW.toml
    let root = TempDir::new().unwrap();
    root.child("workflows/feature/WORKFLOW.toml").write_str("").unwrap();
    root.child("workflows/hotfix/WORKFLOW.toml").write_str("").unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: we list workflows
    let mut result = list_workflows(&entries).unwrap();
    result.sort_by(|a, b| a.name.cmp(&b.name));

    // Then: both names are returned with the correct scope
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "feature");
    assert_eq!(result[0].scope, "project");
    assert_eq!(result[1].name, "hotfix");
}

#[test]
fn list_workflows_returns_empty_when_dir_missing() {
    // Given: an entry whose directory doesn't exist
    let entries = [make_entry("/nonexistent/workflows", "project")];

    // When / Then: empty list, no error
    let result = list_workflows(&entries).unwrap();
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// load_workflow
// ---------------------------------------------------------------------------

#[test]
fn load_workflow_loads_valid_workflow() {
    // Given: a feature workflow dir with WORKFLOW.toml
    let root = TempDir::new().unwrap();
    root.child("workflows/feature/WORKFLOW.toml")
        .write_str(FEATURE_WORKFLOW_TOML)
        .unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: we load "feature"
    let wf = load_workflow(&entries, "feature").unwrap();

    // Then: parsed correctly
    assert_eq!(wf.name, "feature");
    assert_eq!(wf.stages.len(), 2);
}

#[test]
fn load_workflow_resolves_scope_qualified_name() {
    // Given: a workflow in the "project" scope
    let root = TempDir::new().unwrap();
    root.child("workflows/feature/WORKFLOW.toml")
        .write_str(FEATURE_WORKFLOW_TOML)
        .unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: we load "project:feature"
    let wf = load_workflow(&entries, "project:feature").unwrap();

    // Then: found
    assert_eq!(wf.name, "feature");
}

#[test]
fn load_workflow_returns_not_found_for_missing() {
    // Given: an empty workflows dir
    let root = TempDir::new().unwrap();
    root.child("workflows").create_dir_all().unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: we load a nonexistent workflow
    let result = load_workflow(&entries, "nonexistent");

    // Then: WorkflowNotFound
    assert!(
        matches!(result, Err(Error::WorkflowNotFound { ref name }) if name == "nonexistent"),
        "expected WorkflowNotFound, got {result:?}",
    );
}

#[test]
fn load_workflow_returns_ambiguous_when_bare_name_in_multiple_scopes() {
    // Given: "feature" exists in both project and user scopes
    let project = TempDir::new().unwrap();
    project.child("feature/WORKFLOW.toml").write_str("").unwrap();
    let user = TempDir::new().unwrap();
    user.child("feature/WORKFLOW.toml").write_str("").unwrap();

    let entries = [
        make_entry(project.path(), "project"),
        make_entry(user.path(), "user"),
    ];

    // When: we load bare "feature"
    let result = load_workflow(&entries, "feature");

    // Then: AmbiguousWorkflow
    assert!(
        matches!(result, Err(Error::AmbiguousWorkflow { ref name, .. }) if name == "feature"),
        "expected AmbiguousWorkflow, got {result:?}",
    );
}

#[test]
fn load_workflow_returns_parse_failed_for_invalid_toml() {
    // Given: a workflow with malformed TOML
    let root = TempDir::new().unwrap();
    root.child("workflows/broken/WORKFLOW.toml")
        .write_str("this is not valid toml ][[[")
        .unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: we try to load it
    let result = load_workflow(&entries, "broken");

    // Then: WorkflowParseFailed
    assert!(
        matches!(result, Err(Error::WorkflowParseFailed { ref name, .. }) if name == "broken"),
        "expected WorkflowParseFailed, got {result:?}",
    );
}

// ---------------------------------------------------------------------------
// create_workflow_template
// ---------------------------------------------------------------------------

#[test]
fn create_workflow_template_writes_workflow_toml() {
    // Given: an empty agents root
    let agents_root = TempDir::new().unwrap();

    // When: we create a workflow template
    create_workflow_template(agents_root.path(), "my-workflow").unwrap();

    // Then: WORKFLOW.toml exists with the workflow name
    let path = agents_root.path().join("workflows/my-workflow/WORKFLOW.toml");
    assert!(path.exists(), "expected {path:?} to exist");
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(
        contents.contains(r#"name = "my-workflow""#),
        "expected name in template, got:\n{contents}",
    );
}

#[test]
fn create_workflow_template_returns_already_exists_on_duplicate() {
    // Given: already created
    let agents_root = TempDir::new().unwrap();
    create_workflow_template(agents_root.path(), "my-workflow").unwrap();

    // When: try again
    let result = create_workflow_template(agents_root.path(), "my-workflow");

    // Then: WorkflowAlreadyExists
    assert!(
        matches!(result, Err(Error::WorkflowAlreadyExists { ref name }) if name == "my-workflow"),
        "expected WorkflowAlreadyExists, got {result:?}",
    );
}

#[test]
fn create_workflow_template_returns_invalid_id_for_name_with_space() {
    // Given: an agents root
    let agents_root = TempDir::new().unwrap();

    // When: invalid name
    let result = create_workflow_template(agents_root.path(), "has space");

    // Then: InvalidId, no file written
    assert!(
        matches!(result, Err(Error::InvalidId { ref id, .. }) if id == "has space"),
        "expected InvalidId, got {result:?}",
    );
    assert!(
        !agents_root.path().join("workflows/has space/WORKFLOW.toml").exists()
    );
}

// ---------------------------------------------------------------------------
// load_prompt_file
// ---------------------------------------------------------------------------

#[test]
fn load_prompt_file_returns_content() {
    // Given: a workflow dir with a prompt file
    let root = TempDir::new().unwrap();
    root.child("workflows/feature/design.md")
        .write_str("# Design Prompt\nDo the thing.")
        .unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: load the prompt file
    let contents = load_prompt_file(&entries, "feature", "design.md").unwrap();

    // Then: full contents returned
    assert_eq!(contents, "# Design Prompt\nDo the thing.");
}

#[test]
fn load_prompt_file_returns_io_error_for_missing_file() {
    // Given: workflow dir exists but no prompt file
    let root = TempDir::new().unwrap();
    root.child("workflows/feature").create_dir_all().unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: load nonexistent file
    let result = load_prompt_file(&entries, "feature", "missing.md");

    // Then: Io error
    assert!(
        matches!(result, Err(Error::Io(_))),
        "expected Err(Error::Io(_)), got {result:?}",
    );
}

// ---------------------------------------------------------------------------
// find_workflow_dir
// ---------------------------------------------------------------------------

#[test]
fn find_workflow_dir_returns_correct_path() {
    // Given: a workflow dir with WORKFLOW.toml
    let root = TempDir::new().unwrap();
    root.child("workflows/feature/WORKFLOW.toml").write_str("").unwrap();
    let entries = [make_entry(root.child("workflows").path(), "project")];

    // When: find the workflow dir
    let dir = find_workflow_dir(&entries, "feature");

    // Then: correct path
    assert_eq!(dir.unwrap(), root.path().join("workflows/feature"));
}

// ---------------------------------------------------------------------------
// create_flow / load_flow / save_flow
// ---------------------------------------------------------------------------

#[test]
fn create_flow_creates_and_loads_initial_state() {
    // Given: a fresh temp directory as the flow dir
    let flow_dir = TempDir::new().unwrap();

    // When: we create a flow
    create_flow(flow_dir.path(), "feature", "design").unwrap();

    // Then: load succeeds with expected fields
    let flow = load_flow(flow_dir.path()).unwrap();
    assert_eq!(flow.workflow, "feature");
    assert_eq!(flow.current_stage, "design");
    assert!(!flow.archived);
}

#[test]
fn create_flow_returns_already_exists_if_state_toml_present() {
    // Given: flow already created
    let flow_dir = TempDir::new().unwrap();
    create_flow(flow_dir.path(), "feature", "design").unwrap();

    // When: try again
    let result = create_flow(flow_dir.path(), "feature", "design");

    // Then: FlowAlreadyExists
    assert!(
        matches!(result, Err(Error::FlowAlreadyExists { .. })),
        "expected FlowAlreadyExists, got {result:?}",
    );
}

#[test]
fn load_flow_returns_not_found_for_missing_state_toml() {
    // Given: a directory with no flow_state.toml
    let flow_dir = TempDir::new().unwrap();

    // When: load
    let result = load_flow(flow_dir.path());

    // Then: FlowNotFound
    assert!(
        matches!(result, Err(Error::FlowNotFound { .. })),
        "expected FlowNotFound, got {result:?}",
    );
}

#[test]
fn load_flow_returns_parse_failed_for_invalid_toml() {
    // Given: a malformed flow_state.toml
    let flow_dir = TempDir::new().unwrap();
    flow_dir
        .child("flow_state.toml")
        .write_str("this is not valid toml ][[[")
        .unwrap();

    // When: load
    let result = load_flow(flow_dir.path());

    // Then: FlowParseFailed
    assert!(
        matches!(result, Err(Error::FlowParseFailed { .. })),
        "expected FlowParseFailed, got {result:?}",
    );
}

#[test]
fn save_flow_round_trips_through_load_flow() {
    // Given: a temp dir and a FlowState in memory
    let flow_dir = TempDir::new().unwrap();
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

    // When: save then load
    save_flow(flow_dir.path(), &original).unwrap();
    let loaded = load_flow(flow_dir.path()).unwrap();

    // Then: identical
    assert_eq!(loaded, original);
}

// ---------------------------------------------------------------------------
// archive_flow
// ---------------------------------------------------------------------------

#[test]
fn archive_flow_sets_archived_flag_in_state_toml() {
    // Given: an active flow
    let flow_dir = TempDir::new().unwrap();
    create_flow(flow_dir.path(), "feature", "design").unwrap();

    // When: archive
    archive_flow(flow_dir.path()).unwrap();

    // Then: flow_state.toml has archived = true and directory still exists
    let state = load_flow(flow_dir.path()).unwrap();
    assert!(state.archived);
    assert!(flow_dir.path().exists());
}

#[test]
fn archive_flow_returns_not_found_for_nonexistent_flow() {
    // Given: directory with no flow_state.toml
    let flow_dir = TempDir::new().unwrap();

    // When / Then: FlowNotFound
    let result = archive_flow(flow_dir.path());
    assert!(
        matches!(result, Err(Error::FlowNotFound { .. })),
        "expected FlowNotFound, got {result:?}",
    );
}

// ---------------------------------------------------------------------------
// scan_flows
// ---------------------------------------------------------------------------

#[test]
fn scan_flows_finds_state_toml_in_subdirs() {
    // Given: two flow dirs inside a search root
    let search_root = TempDir::new().unwrap();
    let flow_a = search_root.child("2026-04-05/alpha");
    let flow_b = search_root.child("2026-04-05/beta");
    create_flow(flow_a.path(), "feature", "design").unwrap();
    create_flow(flow_b.path(), "hotfix", "patch").unwrap();

    // When: scan
    let summaries = scan_flows(&[search_root.path().to_path_buf()]).unwrap();

    // Then: both flows found
    assert_eq!(summaries.len(), 2);
    let paths: Vec<String> =
        summaries.iter().map(|s| s.path.display().to_string()).collect();
    assert!(
        paths.iter().any(|p| p.ends_with("alpha")),
        "alpha not found in {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p.ends_with("beta")),
        "beta not found in {paths:?}"
    );
}

#[test]
fn scan_flows_excludes_archived_flows_indirectly() {
    // Given: one active and one archived flow
    let search_root = TempDir::new().unwrap();
    let flow_active = search_root.child("active-flow");
    let flow_archived = search_root.child("archived-flow");
    create_flow(flow_active.path(), "feature", "design").unwrap();
    create_flow(flow_archived.path(), "feature", "design").unwrap();
    archive_flow(flow_archived.path()).unwrap();

    // When: scan (returns all flows)
    let all = scan_flows(&[search_root.path().to_path_buf()]).unwrap();

    // Then: both appear; the archived one has archived=true
    assert_eq!(all.len(), 2);
    let archived_entry =
        all.iter().find(|s| s.path.ends_with("archived-flow")).unwrap();
    assert!(archived_entry.archived);
    let active_entry =
        all.iter().find(|s| s.path.ends_with("active-flow")).unwrap();
    assert!(!active_entry.archived);
}

#[test]
fn scan_flows_returns_empty_for_nonexistent_dir() {
    // Given: nonexistent search dir
    let result =
        scan_flows(&[std::path::PathBuf::from("/nonexistent/dir")]).unwrap();
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// put_artifact / artifact_exists / read_artifact
// ---------------------------------------------------------------------------

#[test]
fn put_artifact_copies_file_into_artifact_dir() {
    // Given: an active flow and a source file
    let flow_dir = TempDir::new().unwrap();
    create_flow(flow_dir.path(), "feature", "design").unwrap();

    let src = TempDir::new().unwrap();
    src.child("design.md").write_str("# Design\nHello.").unwrap();

    // When: put artifact
    put_artifact(flow_dir.path(), "design.md", &src.path().join("design.md"))
        .unwrap();

    // Then: file exists with correct content
    let artifact_path = flow_dir.path().join("artifacts/design.md");
    assert!(artifact_path.exists());
    assert_eq!(
        std::fs::read_to_string(&artifact_path).unwrap(),
        "# Design\nHello."
    );
}

#[test]
fn artifact_exists_returns_true_when_present() {
    let flow_dir = TempDir::new().unwrap();
    create_flow(flow_dir.path(), "feature", "design").unwrap();
    let src = TempDir::new().unwrap();
    src.child("design.md").write_str("content").unwrap();
    put_artifact(flow_dir.path(), "design.md", &src.path().join("design.md"))
        .unwrap();

    assert!(artifact_exists(flow_dir.path(), "design.md"));
}

#[test]
fn artifact_exists_returns_false_when_absent() {
    let flow_dir = TempDir::new().unwrap();
    create_flow(flow_dir.path(), "feature", "design").unwrap();

    assert!(!artifact_exists(flow_dir.path(), "missing.md"));
}

#[test]
fn read_artifact_returns_content() {
    let flow_dir = TempDir::new().unwrap();
    create_flow(flow_dir.path(), "feature", "design").unwrap();
    let src = TempDir::new().unwrap();
    src.child("design.md").write_str("# My Design\nDetails here.").unwrap();
    put_artifact(flow_dir.path(), "design.md", &src.path().join("design.md"))
        .unwrap();

    let contents = read_artifact(flow_dir.path(), "design.md").unwrap();
    assert_eq!(contents, "# My Design\nDetails here.");
}

#[test]
fn read_artifact_returns_io_error_for_missing() {
    let flow_dir = TempDir::new().unwrap();
    create_flow(flow_dir.path(), "feature", "design").unwrap();

    let result = read_artifact(flow_dir.path(), "missing.md");
    assert!(
        matches!(result, Err(Error::Io(_))),
        "expected Err(Error::Io(_)), got {result:?}",
    );
}
