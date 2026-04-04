//! Integration tests T1–T7 for the `awf` CLI.
//!
//! Each test invokes the real `awf` binary via [`assert_cmd`] with a temporary
//! directory that contains a `.awf/` tree. [`setup_feature_workflow`] is the
//! shared fixture that populates that tree with the `feature` workflow and its
//! prompt files.
//!
//! # Layout assumed by the tests
//!
//! ```text
//! <tmp>/
//! └── .awf/
//!     └── workflows/
//!         ├── feature.toml
//!         └── feature/
//!             ├── design.md
//!             ├── implement.md
//!             └── review.md
//! ```
//!
//! Every `Command` call sets `.current_dir(&tmp)` so that `find_awf_root` can
//! locate the `.awf/` directory by walking up from the process CWD.

use std::path::Path;

use assert_cmd::Command;
use assert_fs::TempDir;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// T1 — Fixture helper
// ---------------------------------------------------------------------------

/// Feature-workflow TOML content used across all tests that need a workflow.
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

/// Creates the `feature` workflow fixture inside `<tmp>/.awf/`.
///
/// Writes:
/// - `.awf/workflows/feature.toml` — the workflow definition
/// - `.awf/workflows/feature/design.md` — prompt file
/// - `.awf/workflows/feature/implement.md` — prompt file
/// - `.awf/workflows/feature/review.md` — prompt file
fn setup_feature_workflow(tmp: &TempDir) {
    let awf_dir = tmp.path().join(".awf");
    let workflows_dir = awf_dir.join("workflows");
    let prompts_dir = workflows_dir.join("feature");

    std::fs::create_dir_all(&prompts_dir)
        .expect("create .awf/workflows/feature/");

    std::fs::write(workflows_dir.join("feature.toml"), FEATURE_TOML.trim())
        .expect("write feature.toml");

    std::fs::write(
        prompts_dir.join("design.md"),
        "# Design\nWrite a design document.\n",
    )
    .expect("write design.md");

    std::fs::write(
        prompts_dir.join("implement.md"),
        "# Implement\nWrite the implementation.\n",
    )
    .expect("write implement.md");

    std::fs::write(
        prompts_dir.join("review.md"),
        "# Review\nPerform code review.\n",
    )
    .expect("write review.md");
}

/// Creates a minimal workflow that uses inline `run` steps so tests can
/// control pass/fail without relying on real project tools.
///
/// The `pass-step` uses `run = "true"` (always succeeds) and `fail-step` uses
/// `run = "false"` (always fails).  Both live in the single `work` stage.
fn setup_run_workflow(tmp: &TempDir) {
    let awf_dir = tmp.path().join(".awf");
    let workflows_dir = awf_dir.join("workflows");

    std::fs::create_dir_all(&workflows_dir).expect("create .awf/workflows/");

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

    std::fs::write(workflows_dir.join("run-test.toml"), toml.trim())
        .expect("write run-test.toml");
}

/// Returns a [`Command`] builder for the `awf` binary rooted at `tmp`.
///
/// Sets the working directory to `tmp` so `find_awf_root` can locate `.awf/`.
fn awf(tmp: &Path) -> Command {
    let mut cmd = Command::cargo_bin("awf").expect("awf binary");
    cmd.current_dir(tmp);
    cmd
}

// ---------------------------------------------------------------------------
// T2 — `awf workflow new`
// ---------------------------------------------------------------------------

#[test]
fn t2_workflow_new_creates_file() {
    // Given: a temp dir with a bare `.awf/workflows/` directory
    let tmp = TempDir::new().unwrap();
    let awf_dir = tmp.path().join(".awf");
    std::fs::create_dir_all(awf_dir.join("workflows")).unwrap();

    // When: we run `awf workflow new my-wf`
    awf(tmp.path())
        .args(["workflow", "new", "my-wf"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created .awf/workflows/my-wf.toml"));

    // Then: the file exists on disk
    assert!(
        awf_dir.join("workflows").join("my-wf.toml").exists(),
        "my-wf.toml should have been created"
    );
}

#[test]
fn t2_workflow_new_duplicate_errors() {
    // Given: a temp dir with an already-created workflow
    let tmp = TempDir::new().unwrap();
    let awf_dir = tmp.path().join(".awf");
    std::fs::create_dir_all(awf_dir.join("workflows")).unwrap();

    awf(tmp.path()).args(["workflow", "new", "dup"]).assert().success();

    // When: we try to create the same workflow a second time
    // Then: the command exits non-zero with a "already exists" message
    awf(tmp.path())
        .args(["workflow", "new", "dup"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

// ---------------------------------------------------------------------------
// T3 — `awf workflow validate`
// ---------------------------------------------------------------------------

#[test]
fn t3_validate_passes_on_valid_workflow() {
    // Given: the feature workflow fixture
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    // When/Then: validate reports OK
    awf(tmp.path())
        .args(["workflow", "validate", "feature"])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK"));
}

#[test]
fn t3_validate_reports_missing_command_key() {
    // Given: a workflow whose step references an undefined command key
    let tmp = TempDir::new().unwrap();
    let awf_dir = tmp.path().join(".awf");
    std::fs::create_dir_all(awf_dir.join("workflows")).unwrap();

    let bad_toml = r#"
name = "bad"
version = 1

[[stages]]
name = "do-stuff"
steps = [
    { name = "run-thing", type = "command", command = "undefined-key" },
]
"#;

    std::fs::write(awf_dir.join("workflows").join("bad.toml"), bad_toml.trim())
        .unwrap();

    // When/Then: validate exits non-zero and mentions the missing key
    awf(tmp.path())
        .args(["workflow", "validate", "bad"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("undefined-key"));
}

#[test]
fn t3_validate_reports_missing_prompt_file() {
    // Given: a workflow whose agent step references a non-existent prompt file
    let tmp = TempDir::new().unwrap();
    let awf_dir = tmp.path().join(".awf");
    std::fs::create_dir_all(awf_dir.join("workflows")).unwrap();

    let bad_toml = r#"
name = "noprompt"
version = 1

[[stages]]
name = "ask"
steps = [
    { name = "think", type = "agent", prompt = "missing-prompt.md" },
]
"#;

    std::fs::write(
        awf_dir.join("workflows").join("noprompt.toml"),
        bad_toml.trim(),
    )
    .unwrap();

    // When/Then: validate exits non-zero and mentions the missing file
    awf(tmp.path())
        .args(["workflow", "validate", "noprompt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing-prompt.md"));
}

// ---------------------------------------------------------------------------
// T4 — `awf flow new` + `awf flow status`
// ---------------------------------------------------------------------------

#[test]
fn t4_flow_new_and_status() {
    // Given: the feature workflow fixture
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    // When: we create a new flow named "add-oauth"
    awf(tmp.path())
        .args(["flow", "new", "add-oauth", "--workflow", "feature"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add-oauth"));

    // Then: status shows the first stage and the pending step
    awf(tmp.path())
        .args(["flow", "status", "--flow", "add-oauth"])
        .assert()
        .success()
        .stdout(predicate::str::contains("design"))
        .stdout(predicate::str::contains("write-design"))
        .stdout(predicate::str::contains("pending"));
}

// ---------------------------------------------------------------------------
// T5 — Happy path through the design stage
// ---------------------------------------------------------------------------

#[test]
fn t5_happy_path_design_stage() {
    // Given: the feature workflow with a fresh "add-oauth" flow
    let tmp = TempDir::new().unwrap();
    setup_feature_workflow(&tmp);

    awf(tmp.path())
        .args(["flow", "new", "add-oauth", "--workflow", "feature"])
        .assert()
        .success();

    // Write a local design.md so `artifact put` has a real source file.
    let design_file = tmp.path().join("design.md");
    std::fs::write(&design_file, "# My design\n").unwrap();

    // When: put the design artifact
    awf(tmp.path())
        .args([
            "flow",
            "artifact",
            "put",
            "design.md",
            design_file.to_str().unwrap(),
            "--flow",
            "add-oauth",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("design.md"));

    // When: mark the agent step done
    awf(tmp.path())
        .args(["flow", "step", "done", "write-design", "--flow", "add-oauth"])
        .assert()
        .success()
        .stdout(predicate::str::contains("write-design"));

    // When: advance to the next stage
    // Then: we enter "implement"
    awf(tmp.path())
        .args(["flow", "advance", "--flow", "add-oauth"])
        .assert()
        .success()
        .stdout(predicate::str::contains("implement"));
}

// ---------------------------------------------------------------------------
// T6 — `awf flow run` PASS and FAIL
// ---------------------------------------------------------------------------

#[test]
fn t6_flow_run_pass() {
    // Given: the run-test workflow with a fresh flow
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    awf(tmp.path())
        .args(["flow", "new", "run-flow", "--workflow", "run-test"])
        .assert()
        .success();

    // When: we run the always-passing step
    // Then: the output says PASS
    awf(tmp.path())
        .args(["flow", "run", "pass-step", "--flow", "run-flow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));
}

#[test]
fn t6_flow_run_fail() {
    // Given: the run-test workflow with a fresh flow
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    awf(tmp.path())
        .args(["flow", "new", "run-flow", "--workflow", "run-test"])
        .assert()
        .success();

    // When: we run the always-failing step
    // Then: the output says FAIL (the command itself exits 0 because awf
    // records the failure but doesn't propagate a non-zero exit for `run`)
    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", "run-flow"])
        .assert()
        .stdout(predicate::str::contains("FAIL"));
}

#[test]
fn t6_flow_run_fail_then_rerun_reflects_pass() {
    // Given: run the failing step first
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    awf(tmp.path())
        .args(["flow", "new", "retry-flow", "--workflow", "run-test"])
        .assert()
        .success();

    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", "retry-flow"])
        .assert()
        .stdout(predicate::str::contains("FAIL"));

    // Patch the workflow so fail-step now uses `run = "true"`.
    // Instead of mutating the file, we create a separate flow on a workflow
    // whose fail-step has been replaced with a passing inline command.
    let awf_dir = tmp.path().join(".awf");
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
    std::fs::write(
        awf_dir.join("workflows").join("run-test.toml"),
        patched_toml.trim(),
    )
    .unwrap();

    // When: re-run the (now-passing) step
    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", "retry-flow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    // Then: status shows the step as "pass"
    awf(tmp.path())
        .args(["flow", "status", "--flow", "retry-flow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pass"));
}

// ---------------------------------------------------------------------------
// T7 — `awf flow advance` blocked by failed step
// ---------------------------------------------------------------------------

#[test]
fn t7_advance_blocked_by_failed_step() {
    // Given: a flow where a step has failed
    let tmp = TempDir::new().unwrap();
    setup_run_workflow(&tmp);

    awf(tmp.path())
        .args(["flow", "new", "blocker-flow", "--workflow", "run-test"])
        .assert()
        .success();

    // Run pass-step first so it is recorded as passed.
    awf(tmp.path())
        .args(["flow", "run", "pass-step", "--flow", "blocker-flow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    // Run the failing step so it is recorded as failed.
    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", "blocker-flow"])
        .assert()
        .stdout(predicate::str::contains("FAIL"));

    // When: we try to advance
    // Then: advance exits non-zero and mentions the failed step
    awf(tmp.path())
        .args(["flow", "advance", "--flow", "blocker-flow"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fail-step"));

    // Patch the workflow so fail-step passes, then re-run it.
    let awf_dir = tmp.path().join(".awf");
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
    std::fs::write(
        awf_dir.join("workflows").join("run-test.toml"),
        fixed_toml.trim(),
    )
    .unwrap();

    // Re-run fail-step; it now uses `run = "true"` so it passes.
    awf(tmp.path())
        .args(["flow", "run", "fail-step", "--flow", "blocker-flow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS"));

    // When: advance now that both steps have passed
    // The run-test workflow only has one stage ("work") so there is nothing
    // to advance to — the test verifies the blocker was resolved (the earlier
    // call was the one that should fail; now it either succeeds or gives a
    // "last stage" error rather than a "step failed" error).
    let output = awf(tmp.path())
        .args(["flow", "advance", "--flow", "blocker-flow"])
        .output()
        .unwrap();

    // The advance is no longer blocked by a failed step.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("fail-step") || stderr.contains("last stage"),
        "advance should not be blocked by fail-step any more; stderr: {stderr}"
    );
}
