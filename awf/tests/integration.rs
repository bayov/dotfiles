//! Integration tests for the `awf` CLI.
//!
//! Tests invoke the real `awf` binary with a temporary directory that has
//! a `.agents/workflows/` tree and flows stored under `<tmp>/flows/`.
//!
//! # Project layout
//!
//! ```text
//! <tmp>/
//! ├── .agents/
//! │   └── workflows/
//! │       └── feature/
//! │           ├── WORKFLOW.toml
//! │           ├── design.md
//! │           ├── implement.md
//! │           └── review.md
//! ├── .config/
//! │   └── awf.toml          ← flow_search_dirs = ["<tmp>/flows"]
//! └── flows/                ← flow directories live here
//! ```
//!
//! `find_project_root` walks up from the CWD (`tmp`) looking for `.agents/`.

use std::path::Path;
use std::path::PathBuf;

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// Feature-workflow TOML used across most tests.
const FEATURE_TOML: &str = r#"
name = "feature"
version = 1
description = "Ship a new feature from design through merge."

[commands]
test = "cargo test"
lint = "cargo clippy -- -D warnings"

[[stages]]
name = "design"
description = "Produce a design doc."
produces = ["design.md"]
steps = [
    { name = "write-design", type = "agent", prompt = "design.md" },
]

[[stages]]
name = "implement"
description = "Write the code."
requires = ["design.md"]
steps = [
    { name = "write-code", type = "agent", prompt = "implement.md" },
    { name = "test", type = "command", command = "test" },
]

[[stages]]
name = "review"
description = "Code review and integration."
repeatable = true
produces = ["changelog-entry.md"]
steps = [
    { name = "pre-review-test", type = "command", command = "test" },
    { name = "do-review", type = "agent", prompt = "review.md" },
]

[[stages]]
name = "done"
description = "Complete."
"#;

/// Creates the `feature` workflow fixture inside `<tmp>/.agents/`.
fn setup_feature_workflow(tmp: &TempDir) {
    let workflow_dir = tmp.path().join(".agents/workflows/feature");
    std::fs::create_dir_all(&workflow_dir)
        .expect("create .agents/workflows/feature/");

    std::fs::write(workflow_dir.join("WORKFLOW.toml"), FEATURE_TOML.trim())
        .expect("write WORKFLOW.toml");

    std::fs::write(
        workflow_dir.join("design.md"),
        "# Design\nWrite a design document.\n",
    )
    .expect("write design.md");

    std::fs::write(
        workflow_dir.join("implement.md"),
        "# Implement\nWrite the implementation.\n",
    )
    .expect("write implement.md");

    std::fs::write(
        workflow_dir.join("review.md"),
        "# Review\nPerform code review.\n",
    )
    .expect("write review.md");

    // Write project config so `awf flow list` can find flows.
    setup_flow_config(tmp);
}

/// Creates a minimal workflow that uses inline `run` steps.
fn setup_run_workflow(tmp: &TempDir) {
    let workflow_dir = tmp.path().join(".agents/workflows/run-test");
    std::fs::create_dir_all(&workflow_dir)
        .expect("create run-test workflow dir");

    let toml = r#"
name = "run-test"
version = 1

[[stages]]
name = "work"
steps = [
    { name = "pass-step", type = "command", run = "true" },
    { name = "fail-step", type = "command", run = "false" },
]
"#;

    std::fs::write(workflow_dir.join("WORKFLOW.toml"), toml.trim())
        .expect("write run-test WORKFLOW.toml");

    setup_flow_config(tmp);
}

/// Writes `.config/awf.toml` pointing `flow_search_dirs` at `<tmp>/flows/`.
fn setup_flow_config(tmp: &TempDir) {
    let config_dir = tmp.path().join(".config");
    std::fs::create_dir_all(&config_dir).expect("create .config/");
    let flows_dir = tmp.path().join("flows");
    std::fs::create_dir_all(&flows_dir).expect("create flows/");
    std::fs::write(
        config_dir.join("awf.toml"),
        format!("flow_search_dirs = [{:?}]\n", flows_dir.to_string_lossy()),
    )
    .expect("write .config/awf.toml");
}

/// Returns the path for a named flow under `<tmp>/flows/`.
fn flow_path(tmp: &TempDir, name: &str) -> PathBuf {
    tmp.path().join("flows").join(name)
}

/// Returns a [`Command`] builder for the `awf` binary rooted at `tmp`.
fn awf(tmp: &Path) -> Command {
    let mut cmd = Command::cargo_bin("awf").expect("awf binary");
    cmd.current_dir(tmp);
    cmd
}

/// Helper: create a flow at `<tmp>/flows/<name>` via the CLI.
fn create_flow(tmp: &TempDir, name: &str, workflow: &str) {
    awf(tmp.path())
        .args([
            "flow",
            "new",
            "--flow",
            flow_path(tmp, name).to_str().unwrap(),
            "--workflow",
            workflow,
        ])
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// `awf workflow new`
// ---------------------------------------------------------------------------

#[test]
fn t2_workflow_new_creates_file() {
    // Given: a temp dir with a bare `.agents/workflows/` directory
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".agents/workflows")).unwrap();

    // When: we run `awf workflow new my-wf`
    awf(tmp.path())
        .args(["workflow", "new", "my-wf"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            ".agents/workflows/my-wf/WORKFLOW.toml",
        ));

    // Then: the WORKFLOW.toml exists on disk
    assert!(
        tmp.path().join(".agents/workflows/my-wf/WORKFLOW.toml").exists(),
        "WORKFLOW.toml should have been created"
    );
}

#[test]
fn t2_workflow_new_duplicate_errors() {
    // Given: a temp dir with an already-created workflow
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join(".agents/workflows")).unwrap();

    awf(tmp.path()).args(["workflow", "new", "dup"]).assert().success();

    // When: try to create the same workflow a second time
    // Then: exits non-zero with "already exists"
    awf(tmp.path())
        .args(["workflow", "new", "dup"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// ---------------------------------------------------------------------------
// `awf workflow validate`
// ---------------------------------------------------------------------------

#[test]
fn t3_validate_passes_on_valid_workflow() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    awf(tmp.path())
        .args(["workflow", "validate", "feature"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK"));
}

#[test]
fn t3_validate_reports_missing_command_key() {
    let tmp = TempDir::new().unwrap();
    let workflow_dir = tmp.path().join(".agents/workflows/bad");
    std::fs::create_dir_all(&workflow_dir).unwrap();

    let bad_toml = r#"
name = "bad"
version = 1

[[stages]]
name = "do-stuff"
steps = [
    { name = "run-thing", type = "command", command = "undefined-key" },
]
"#;

    std::fs::write(workflow_dir.join("WORKFLOW.toml"), bad_toml.trim())
        .unwrap();

    awf(tmp.path())
        .args(["workflow", "validate", "bad"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("undefined-key"));
}

#[test]
fn t3_validate_reports_missing_prompt_file() {
    let tmp = TempDir::new().unwrap();
    let workflow_dir = tmp.path().join(".agents/workflows/noprompt");
    std::fs::create_dir_all(&workflow_dir).unwrap();

    let bad_toml = r#"
name = "noprompt"
version = 1

[[stages]]
name = "ask"
steps = [
    { name = "think", type = "agent", prompt = "missing-prompt.md" },
]
"#;

    std::fs::write(workflow_dir.join("WORKFLOW.toml"), bad_toml.trim())
        .unwrap();

    awf(tmp.path())
        .args(["workflow", "validate", "noprompt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing-prompt.md"));
}

// ---------------------------------------------------------------------------
// `awf flow new` + `awf flow status`
// ---------------------------------------------------------------------------

#[test]
fn t4_flow_new_and_status() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "add-oauth");

    awf(tmp.path())
        .args([
            "flow",
            "new",
            "--flow",
            fp.to_str().unwrap(),
            "--workflow",
            "feature",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("add-oauth"));

    awf(tmp.path())
        .args(["flow", "status", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("design"))
        .stdout(predicate::str::contains("write-design"))
        .stdout(predicate::str::contains("pending"));
}

// ---------------------------------------------------------------------------
// Happy path through the design stage
// ---------------------------------------------------------------------------

#[test]
fn t5_happy_path_design_stage() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "add-oauth");
    create_flow(&tmp, "add-oauth", "feature");

    let design_file = tmp.path().join("design.md");
    std::fs::write(&design_file, "# My design\n").unwrap();

    awf(tmp.path())
        .args([
            "flow",
            "artifact",
            "put",
            "design.md",
            design_file.to_str().unwrap(),
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("design.md"));

    awf(tmp.path())
        .args([
            "flow",
            "step",
            "done",
            "write-design",
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("write-design"));

    awf(tmp.path())
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("implement"));
}

// ---------------------------------------------------------------------------
// `awf flow run` PASS and FAIL
// ---------------------------------------------------------------------------

#[test]
fn t6_flow_run_pass() {
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    let fp = flow_path(&tmp, "run-flow");
    create_flow(&tmp, "run-flow", "run-test");

    awf(tmp.path())
        .args(["flow", "run", "pass-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn t6_flow_run_fail() {
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    let fp = flow_path(&tmp, "run-flow");
    create_flow(&tmp, "run-flow", "run-test");

    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("FAIL"));
}

#[test]
fn t6_flow_run_fail_then_rerun_reflects_pass() {
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    let fp = flow_path(&tmp, "retry-flow");
    create_flow(&tmp, "retry-flow", "run-test");

    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("FAIL"));

    // Patch workflow so fail-step now passes.
    let workflow_dir = tmp.path().join(".agents/workflows/run-test");
    let patched_toml = r#"
name = "run-test"
version = 1

[[stages]]
name = "work"
steps = [
    { name = "pass-step", type = "command", run = "true" },
    { name = "fail-step", type = "command", run = "true" },
]
"#;
    std::fs::write(workflow_dir.join("WORKFLOW.toml"), patched_toml.trim())
        .unwrap();

    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    awf(tmp.path())
        .args(["flow", "status", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("pass"));
}

// ---------------------------------------------------------------------------
// `awf flow advance` blocked by failed step
// ---------------------------------------------------------------------------

#[test]
fn t7_advance_blocked_by_failed_step() {
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    let fp = flow_path(&tmp, "blocker-flow");
    create_flow(&tmp, "blocker-flow", "run-test");

    awf(tmp.path())
        .args(["flow", "run", "pass-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("FAIL"));

    // Advance should fail because fail-step failed.
    awf(tmp.path())
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fail-step"));

    // Fix the workflow.
    let workflow_dir = tmp.path().join(".agents/workflows/run-test");
    let fixed_toml = r#"
name = "run-test"
version = 1

[[stages]]
name = "work"
steps = [
    { name = "pass-step", type = "command", run = "true" },
    { name = "fail-step", type = "command", run = "true" },
]
"#;
    std::fs::write(workflow_dir.join("WORKFLOW.toml"), fixed_toml.trim())
        .unwrap();

    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    let output = awf(tmp.path())
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("fail-step") || stderr.contains("last stage"),
        "advance should not be blocked by fail-step any more; stderr: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// `awf flow advance` blocked by missing artifact
// ---------------------------------------------------------------------------

#[test]
fn t8_advance_blocked_by_missing_artifact_then_unblocked() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "add-oauth");
    create_flow(&tmp, "add-oauth", "feature");

    // Mark step done but skip putting the artifact.
    awf(tmp.path())
        .args([
            "flow",
            "step",
            "done",
            "write-design",
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Advance should fail with missing artifact.
    awf(tmp.path())
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("design.md"));

    // Put the artifact then advance.
    let design_file = tmp.path().join("design.md");
    std::fs::write(&design_file, "# My design\n").unwrap();

    awf(tmp.path())
        .args([
            "flow",
            "artifact",
            "put",
            "design.md",
            design_file.to_str().unwrap(),
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success();

    awf(tmp.path())
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("implement"));
}

// ---------------------------------------------------------------------------
// `requires` gate
// ---------------------------------------------------------------------------

const REQUIRES_GATE_TOML: &str = r#"
name = "requires-gate"
version = 1

[[stages]]
name = "start"
steps = [
    { name = "do-thing", type = "command", run = "true" },
]

[[stages]]
name = "finish"
requires = ["special.md"]
steps = []
"#;

fn setup_requires_gate_workflow(tmp: &TempDir) {
    let workflow_dir = tmp.path().join(".agents/workflows/requires-gate");
    std::fs::create_dir_all(&workflow_dir).expect("create requires-gate dir");
    std::fs::write(
        workflow_dir.join("WORKFLOW.toml"),
        REQUIRES_GATE_TOML.trim(),
    )
    .expect("write requires-gate WORKFLOW.toml");
    setup_flow_config(tmp);
}

#[test]
fn t9_advance_blocked_by_missing_requires_artifact() {
    let tmp = TempDir::new().unwrap();
    setup_requires_gate_workflow(&tmp);

    let fp = flow_path(&tmp, "gate-flow");
    create_flow(&tmp, "gate-flow", "requires-gate");

    awf(tmp.path())
        .args(["flow", "run", "do-thing", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    awf(tmp.path())
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("special.md"));
}

// ---------------------------------------------------------------------------
// `--force` bypasses step-completion gate
// ---------------------------------------------------------------------------

#[test]
fn t10_force_advance_bypasses_incomplete_steps() {
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    let fp = flow_path(&tmp, "force-flow");
    create_flow(&tmp, "force-flow", "run-test");

    // Without --force: fails because steps are incomplete.
    awf(tmp.path())
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .assert()
        .failure();

    // With --force: step gate bypassed.
    let output = awf(tmp.path())
        .args(["flow", "advance", "--force", "--flow", fp.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stderr.contains("pass-step") && !stderr.contains("fail-step"),
        "force advance should not be blocked by incomplete steps; \
         stderr: {stderr}"
    );

    let mentions_stage_or_last =
        stdout.contains("Entered") || stderr.contains("last stage");
    assert!(
        mentions_stage_or_last,
        "expected 'Entered' in stdout or 'last stage' in stderr; \
         stdout: {stdout}; stderr: {stderr}"
    );

    awf(tmp.path())
        .args(["flow", "history", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("work"));
}

// ---------------------------------------------------------------------------
// Duplicate flow path blocked
// ---------------------------------------------------------------------------

#[test]
fn t11_duplicate_flow_path_blocked() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "add-oauth");

    // First creation succeeds.
    awf(tmp.path())
        .args([
            "flow",
            "new",
            "--flow",
            fp.to_str().unwrap(),
            "--workflow",
            "feature",
        ])
        .assert()
        .success();

    // Second creation at the same path fails.
    awf(tmp.path())
        .args([
            "flow",
            "new",
            "--flow",
            fp.to_str().unwrap(),
            "--workflow",
            "feature",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    // Archive then try again — still blocked (flow_state.toml exists).
    awf(tmp.path())
        .args(["flow", "archive", "--flow", fp.to_str().unwrap()])
        .assert()
        .success();

    awf(tmp.path())
        .args([
            "flow",
            "new",
            "--flow",
            fp.to_str().unwrap(),
            "--workflow",
            "feature",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// ---------------------------------------------------------------------------
// `awf flow next` output
// ---------------------------------------------------------------------------

#[test]
fn t12_flow_next_failing_step_hints_run() {
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    let fp = flow_path(&tmp, "next-flow");
    create_flow(&tmp, "next-flow", "run-test");

    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .stdout(predicate::str::contains("FAIL"));

    awf(tmp.path())
        .args(["flow", "next", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("awf flow run"));
}

#[test]
fn t12_flow_next_incomplete_agent_step_hints_done() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "next-agent");
    create_flow(&tmp, "next-agent", "feature");

    awf(tmp.path())
        .args(["flow", "next", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mark complete with"));
}

#[test]
fn t12_flow_next_missing_artifact_mentions_artifact_name() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "next-artifact");
    create_flow(&tmp, "next-artifact", "feature");

    awf(tmp.path())
        .args([
            "flow",
            "step",
            "done",
            "write-design",
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success();

    awf(tmp.path())
        .args(["flow", "next", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("design.md"));
}

#[test]
fn t12_flow_next_can_advance_mentions_advance_command() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "next-advance");
    create_flow(&tmp, "next-advance", "feature");

    let design_file = tmp.path().join("design.md");
    std::fs::write(&design_file, "# My design\n").unwrap();

    awf(tmp.path())
        .args([
            "flow",
            "artifact",
            "put",
            "design.md",
            design_file.to_str().unwrap(),
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success();

    awf(tmp.path())
        .args([
            "flow",
            "step",
            "done",
            "write-design",
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success();

    awf(tmp.path())
        .args(["flow", "next", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("awf flow advance"));
}

// ---------------------------------------------------------------------------
// `awf flow status --output-format json`
// ---------------------------------------------------------------------------

#[test]
fn t13_flow_status_json_output_is_valid() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "add-oauth");
    create_flow(&tmp, "add-oauth", "feature");

    awf(tmp.path())
        .args([
            "flow",
            "step",
            "done",
            "write-design",
            "--flow",
            fp.to_str().unwrap(),
        ])
        .assert()
        .success();

    let output = awf(tmp.path())
        .args([
            "flow",
            "status",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8(output).expect("valid utf-8");
    let value: serde_json::Value = serde_json::from_str(&json_str)
        .expect("valid JSON from status --output-format json");

    assert!(
        value.get("flow").is_some(),
        "JSON should contain 'flow'; got: {json_str}"
    );
    assert!(
        value.get("stage").is_some(),
        "JSON should contain 'stage'; got: {json_str}"
    );
    assert!(
        value.get("steps").is_some(),
        "JSON should contain 'steps'; got: {json_str}"
    );
}

// ---------------------------------------------------------------------------
// `awf flow archive` — marks archived; `list` hides it; `list --all` shows it
// ---------------------------------------------------------------------------

#[test]
fn t14_archive_and_list() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "add-oauth");
    create_flow(&tmp, "add-oauth", "feature");

    // Flow appears in normal list.
    awf(tmp.path())
        .args(["flow", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add-oauth"));

    // Archive the flow.
    awf(tmp.path())
        .args(["flow", "archive", "--flow", fp.to_str().unwrap()])
        .assert()
        .success();

    // After archiving, `status` still succeeds (path-based load).
    awf(tmp.path())
        .args(["flow", "status", "--flow", fp.to_str().unwrap()])
        .assert()
        .success();

    // Plain `list` does NOT show "add-oauth".
    let list_out = awf(tmp.path())
        .args(["flow", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let list_str = String::from_utf8(list_out).expect("valid utf-8");
    assert!(
        !list_str.contains("add-oauth"),
        "plain list should not show archived flow; got: {list_str}"
    );

    // `list --all` DOES show "add-oauth" as archived.
    awf(tmp.path())
        .args(["flow", "list", "--all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add-oauth"))
        .stdout(predicate::str::contains("yes"));
}

// ---------------------------------------------------------------------------
// Flow state stored at <flow_dir>/flow_state.toml
// ---------------------------------------------------------------------------

#[test]
fn t15_flow_state_stored_in_flow_dir() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "path-check");

    awf(tmp.path())
        .args([
            "flow",
            "new",
            "--flow",
            fp.to_str().unwrap(),
            "--workflow",
            "feature",
        ])
        .assert()
        .success();

    // State file is at <flow_dir>/flow_state.toml
    let state_path = fp.join("flow_state.toml");
    assert!(state_path.exists(), "flow state should be at {state_path:?}");
}
