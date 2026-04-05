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
use rstest::rstest;

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
    setup_flow_config_with_commands(tmp, &[]);
}

/// Writes `.config/awf.toml` with `flow_search_dirs` and optional repo-wide
/// `[commands]` entries (each element is a `"key = \"shell\""` string).
fn setup_flow_config_with_commands(tmp: &TempDir, commands: &[(&str, &str)]) {
    let config_dir = tmp.path().join(".config");
    std::fs::create_dir_all(&config_dir).expect("create .config/");
    let flows_dir = tmp.path().join("flows");
    std::fs::create_dir_all(&flows_dir).expect("create flows/");

    let mut contents =
        format!("flow_search_dirs = [{:?}]\n", flows_dir.to_string_lossy());

    if !commands.is_empty() {
        use std::fmt::Write as _;
        contents.push_str("\n[commands]\n");
        for (key, shell) in commands {
            let _ = writeln!(contents, "{key} = {shell:?}");
        }
    }

    std::fs::write(config_dir.join("awf.toml"), contents)
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
fn workflow_new_creates_file() {
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
fn workflow_new_duplicate_errors() {
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
fn workflow_validate_passes_on_valid_workflow() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    awf(tmp.path())
        .args(["workflow", "validate", "feature"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK"));
}

#[test]
fn workflow_validate_reports_missing_command_key() {
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
fn workflow_validate_reports_missing_prompt_file() {
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
fn flow_new_and_status() {
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
fn flow_happy_path_design_stage() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "add-oauth");
    create_flow(&tmp, "add-oauth", "feature");

    let artifact_dir = fp.join("artifacts");
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("design.md"), "# My design\n").unwrap();

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
fn flow_run_pass() {
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
fn flow_run_fail() {
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
fn flow_run_fail_then_rerun_reflects_pass() {
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
fn advance_blocked_by_failed_step() {
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
fn advance_blocked_by_missing_artifact_then_unblocked() {
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

    // Write the artifact directly then advance.
    let artifact_dir = fp.join("artifacts");
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("design.md"), "# My design\n").unwrap();

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
fn advance_blocked_by_missing_requires_artifact() {
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
fn force_advance_bypasses_incomplete_steps() {
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
fn duplicate_flow_path_blocked() {
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
fn flow_next_failing_step_hints_run() {
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
fn flow_next_incomplete_agent_step_hints_done() {
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
fn flow_next_missing_artifact_mentions_artifact_name() {
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
fn flow_next_can_advance_mentions_advance_command() {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "next-advance");
    create_flow(&tmp, "next-advance", "feature");

    let artifact_dir = fp.join("artifacts");
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("design.md"), "# My design\n").unwrap();

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
fn flow_status_json_output_is_valid() {
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
fn archive_and_list() {
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
// `awf flow validate`
// ---------------------------------------------------------------------------

#[test]
fn flow_validate_passes_when_no_stages_completed() {
    // Given: a fresh flow in the design stage with no artifacts on disk
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "validate-empty");
    create_flow(&tmp, "validate-empty", "feature");

    // When: validate is run with an empty artifacts directory
    // Then: succeeds — nothing is expected yet and nothing is redundant
    awf(tmp.path())
        .args(["flow", "validate", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn flow_validate_passes_when_completed_stage_artifacts_present() {
    // Given: design stage completed and design.md present in artifacts/
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "validate-ok");
    create_flow(&tmp, "validate-ok", "feature");

    let artifact_dir = fp.join("artifacts");
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("design.md"), "# Design\n").unwrap();

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
        .args(["flow", "advance", "--flow", fp.to_str().unwrap()])
        .assert()
        .success();

    // When: validate is run after advancing past design
    // Then: succeeds — design.md is present as expected
    awf(tmp.path())
        .args(["flow", "validate", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn flow_validate_fails_for_missing_artifact() {
    // Given: a workflow where stage 2 does not `requires` the stage-1 artifact
    // so --force can advance without the artifact being present.
    let tmp = TempDir::new().unwrap();

    let workflow_dir = tmp.path().join(".agents/workflows/produces-only");
    std::fs::create_dir_all(&workflow_dir).unwrap();
    std::fs::write(
        workflow_dir.join("WORKFLOW.toml"),
        r#"name = "produces-only"
version = 1

[[stages]]
name = "draft"
produces = ["draft.md"]
steps = [{ name = "write", type = "agent", instructions = "Write it." }]

[[stages]]
name = "done"
steps = []
"#,
    )
    .unwrap();
    setup_flow_config(&tmp);

    let fp = flow_path(&tmp, "validate-missing");
    create_flow(&tmp, "validate-missing", "produces-only");

    // Force-advance past draft without writing draft.md.
    awf(tmp.path())
        .args(["flow", "advance", "--force", "--flow", fp.to_str().unwrap()])
        .assert()
        .success();

    // When: validate is run
    // Then: fails and names the missing artifact
    awf(tmp.path())
        .args(["flow", "validate", "--flow", fp.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing artifact: draft.md"));
}

#[test]
fn flow_validate_fails_for_redundant_artifact() {
    // Given: flow is in design stage; an undeclared file is written to
    // artifacts/
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "validate-redundant");
    create_flow(&tmp, "validate-redundant", "feature");

    let artifact_dir = fp.join("artifacts");
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("stray.txt"), "oops\n").unwrap();

    // When: validate is run
    // Then: fails and names the redundant artifact
    awf(tmp.path())
        .args(["flow", "validate", "--flow", fp.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("redundant artifact: stray.txt"));
}

// ---------------------------------------------------------------------------
// Flow state stored at <flow_dir>/flow_state.toml
// ---------------------------------------------------------------------------

#[test]
fn flow_state_stored_in_flow_dir() {
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

// ---------------------------------------------------------------------------
// Minimal workflow fixture (no steps, no artifacts) used for advance tests.
// ---------------------------------------------------------------------------

fn setup_minimal_workflow(tmp: &TempDir) {
    let workflow_dir = tmp.path().join(".agents/workflows/minimal");
    std::fs::create_dir_all(&workflow_dir)
        .expect("create .agents/workflows/minimal/");
    std::fs::write(
        workflow_dir.join("WORKFLOW.toml"),
        "name = \"minimal\"\nversion = 1\n\n\
         [[stages]]\nname = \"work\"\n\n\
         [[stages]]\nname = \"done\"\n",
    )
    .expect("write minimal WORKFLOW.toml");
    setup_flow_config(tmp);
}

// ---------------------------------------------------------------------------
// Helper: parse machine-readable output into a `serde_json::Value`.
//
// Returns `None` for the `"human"` format so callers can apply string
// assertions instead.  For all machine formats the returned value is a
// `serde_json::Value`, making assertions format-agnostic.
// ---------------------------------------------------------------------------

fn parse_machine_output(output: &[u8], fmt: &str) -> Option<serde_json::Value> {
    let s = std::str::from_utf8(output).expect("valid utf-8");
    match fmt {
        "json" => {
            let val: serde_json::Value = serde_json::from_str(s)
                .unwrap_or_else(|e| {
                    panic!("invalid JSON ({fmt}): {e}\n---\n{s}\n---")
                });
            Some(val)
        }
        "toml" => {
            let val: toml::Value = s.parse().unwrap_or_else(|e| {
                panic!("invalid TOML ({fmt}): {e}\n---\n{s}\n---")
            });
            Some(
                serde_json::to_value(&val)
                    .expect("TOML value to serde_json::Value"),
            )
        }
        "xml" => {
            assert!(
                s.starts_with("<data>\n"),
                "XML output must start with '<data>\\n': {s}"
            );
            assert!(
                s.trim_end().ends_with("</data>"),
                "XML output must end with '</data>': {s}"
            );
            let inner = s
                .trim_start_matches("<data>\n")
                .trim_end_matches('\n')
                .trim_end_matches("</data>");
            let val: serde_json::Value = serde_json::from_str(inner)
                .unwrap_or_else(|e| {
                    panic!(
                        "invalid JSON inside XML envelope: {e}\n---\n{inner}\n---"
                    )
                });
            Some(val)
        }
        "human" => None,
        other => panic!("unknown format: {other}"),
    }
}

// ---------------------------------------------------------------------------
// `awf workflow new --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn workflow_new_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp); // sets up .agents/ and config

    let out = awf(tmp.path())
        .args(["workflow", "new", "fmt-wf", "--output-format", fmt])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("path").is_some(),
            "expected 'path' field in {fmt} output: {val}"
        );
        assert!(
            val.get("name").is_some(),
            "expected 'name' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("Created"),
            "human output should contain 'Created': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf workflow validate --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn workflow_validate_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let out = awf(tmp.path())
        .args(["workflow", "validate", "feature", "--output-format", fmt])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("name").is_some(),
            "expected 'name' field in {fmt} output: {val}"
        );
        assert!(
            val.get("status").is_some(),
            "expected 'status' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("OK"), "human output should contain 'OK': {s}");
    }
}

// ---------------------------------------------------------------------------
// `awf workflow list --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn workflow_list_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let out = awf(tmp.path())
        .args(["workflow", "list", "--output-format", fmt])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("workflows").is_some(),
            "expected 'workflows' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("feature"),
            "human output should contain 'feature': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow new --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_new_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-new");
    let out = awf(tmp.path())
        .args([
            "flow",
            "new",
            "--flow",
            fp.to_str().unwrap(),
            "--workflow",
            "feature",
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("path").is_some(),
            "expected 'path' field in {fmt} output: {val}"
        );
        assert!(
            val.get("workflow").is_some(),
            "expected 'workflow' field in {fmt} output: {val}"
        );
        assert!(
            val.get("stage").is_some(),
            "expected 'stage' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("Created flow"),
            "human output should contain 'Created flow': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow list --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_list_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);
    create_flow(&tmp, "fmt-list", "feature");

    let out = awf(tmp.path())
        .args(["flow", "list", "--output-format", fmt])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("flows").is_some(),
            "expected 'flows' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("fmt-list"),
            "human output should contain 'fmt-list': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow status --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_status_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-status");
    create_flow(&tmp, "fmt-status", "feature");

    let out = awf(tmp.path())
        .args([
            "flow",
            "status",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("flow").is_some(),
            "expected 'flow' field in {fmt} output: {val}"
        );
        assert!(
            val.get("stage").is_some(),
            "expected 'stage' field in {fmt} output: {val}"
        );
        assert!(
            val.get("steps").is_some(),
            "expected 'steps' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("Stage:"),
            "human output should contain 'Stage:': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow next --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_next_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-next");
    create_flow(&tmp, "fmt-next", "feature");

    let out = awf(tmp.path())
        .args([
            "flow",
            "next",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("action").is_some(),
            "expected 'action' field in {fmt} output: {val}"
        );
        assert!(
            val.get("message").is_some(),
            "expected 'message' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(!s.is_empty(), "human output should not be empty: {s}");
    }
}

// ---------------------------------------------------------------------------
// `awf flow run --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_run_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-run");
    create_flow(&tmp, "fmt-run", "run-test");

    let out = awf(tmp.path())
        .args([
            "flow",
            "run",
            "pass-step",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("step").is_some(),
            "expected 'step' field in {fmt} output: {val}"
        );
        assert!(
            val.get("status").is_some(),
            "expected 'status' field in {fmt} output: {val}"
        );
        assert!(
            val.get("duration_ms").is_some(),
            "expected 'duration_ms' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(s.contains("PASS"), "human output should contain 'PASS': {s}");
    }
}

// ---------------------------------------------------------------------------
// `awf flow step done --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_step_done_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-done");
    create_flow(&tmp, "fmt-done", "feature");

    let out = awf(tmp.path())
        .args([
            "flow",
            "step",
            "done",
            "write-design",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("step").is_some(),
            "expected 'step' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("marked complete"),
            "human output should contain 'marked complete': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow validate --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_validate_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-validate");
    create_flow(&tmp, "fmt-validate", "feature");

    let out = awf(tmp.path())
        .args([
            "flow",
            "validate",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("valid").is_some(),
            "expected 'valid' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("valid"),
            "human output should contain 'valid': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow advance --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_advance_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_minimal_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-advance");
    create_flow(&tmp, "fmt-advance", "minimal");

    let out = awf(tmp.path())
        .args([
            "flow",
            "advance",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("stage").is_some(),
            "expected 'stage' field in {fmt} output: {val}"
        );
        assert!(
            val.get("is_final").is_some(),
            "expected 'is_final' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("Entered stage:"),
            "human output should contain 'Entered stage:': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow history --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_history_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-history");
    create_flow(&tmp, "fmt-history", "feature");

    let out = awf(tmp.path())
        .args([
            "flow",
            "history",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("entries").is_some(),
            "expected 'entries' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("entered"),
            "human output should contain 'entered': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// `awf flow archive --output-format <fmt>`
// ---------------------------------------------------------------------------

#[rstest]
#[case::human("human")]
#[case::json("json")]
#[case::toml("toml")]
#[case::xml("xml")]
fn flow_archive_output_format(#[case] fmt: &str) {
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    let fp = flow_path(&tmp, "fmt-archive");
    create_flow(&tmp, "fmt-archive", "feature");

    let out = awf(tmp.path())
        .args([
            "flow",
            "archive",
            "--flow",
            fp.to_str().unwrap(),
            "--output-format",
            fmt,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    if let Some(val) = parse_machine_output(&out, fmt) {
        assert!(
            val.get("path").is_some(),
            "expected 'path' field in {fmt} output: {val}"
        );
    } else {
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("Archived flow:"),
            "human output should contain 'Archived flow:': {s}"
        );
    }
}

// ---------------------------------------------------------------------------
// Repo-wide commands in `.config/awf.toml`
// ---------------------------------------------------------------------------

/// Creates a workflow with no `[commands]` table — relies entirely on the
/// repo-level command defined in `.config/awf.toml`.
fn setup_repo_command_workflow(tmp: &TempDir) {
    let workflow_dir = tmp.path().join(".agents/workflows/repo-cmd-test");
    std::fs::create_dir_all(&workflow_dir)
        .expect("create repo-cmd-test workflow dir");

    // Workflow has no [commands] section; `true-step` references a `repo-true`
    // alias that must come from the repo config.
    let toml = r#"
name = "repo-cmd-test"
version = 1

[[stages]]
name = "work"
steps = [
    { name = "true-step", type = "command", command = "repo-true" },
]
"#;

    std::fs::write(workflow_dir.join("WORKFLOW.toml"), toml.trim())
        .expect("write repo-cmd-test WORKFLOW.toml");

    setup_flow_config_with_commands(tmp, &[("repo-true", "true")]);
}

#[test]
fn flow_run_resolves_command_from_repo_config() {
    // Given: a workflow with no [commands] and a repo-level command "repo-true"
    let tmp = TempDir::new().unwrap();
    setup_repo_command_workflow(&tmp);

    let fp = flow_path(&tmp, "repo-flow");
    create_flow(&tmp, "repo-flow", "repo-cmd-test");

    // When: we run the step that uses the repo-level command alias
    // Then: it resolves and succeeds
    awf(tmp.path())
        .args(["flow", "run", "true-step", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn workflow_command_takes_priority_over_repo_command() {
    // Given: a workflow that defines "shared" as "true", and the repo config
    // also defines "shared" as "false" — the workflow definition must win.
    let tmp = TempDir::new().unwrap();

    let workflow_dir = tmp.path().join(".agents/workflows/priority-test");
    std::fs::create_dir_all(&workflow_dir).expect("create priority-test dir");

    let toml = r#"
name = "priority-test"
version = 1

[commands]
shared = "true"

[[stages]]
name = "work"
steps = [
    { name = "check", type = "command", command = "shared" },
]
"#;
    std::fs::write(workflow_dir.join("WORKFLOW.toml"), toml.trim())
        .expect("write WORKFLOW.toml");

    // Repo config defines the same alias with "false" (would fail if used).
    setup_flow_config_with_commands(&tmp, &[("shared", "false")]);

    let fp = flow_path(&tmp, "priority-flow");
    create_flow(&tmp, "priority-flow", "priority-test");

    // When: we run the step
    // Then: the workflow-level "true" wins and the step passes
    awf(tmp.path())
        .args(["flow", "run", "check", "--flow", fp.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}
