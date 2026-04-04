//! Tests for [`super::run_command`].

use super::run_command;

#[test]
fn known_pass_command_exits_zero() {
    // When: we run `true`, which always exits 0
    let result = run_command("true").expect("spawn should succeed");

    // Then: exit code is 0 and duration is non-negative
    assert_eq!(result.exit_code, 0);
    // duration_ms is u64, so it is always >= 0; just verify it is accessible
    let _ = result.duration_ms;
}

#[test]
fn known_fail_command_exits_one() {
    // When: we run `false`, which always exits 1
    let result = run_command("false").expect("spawn should succeed");

    // Then: exit code is 1
    assert_eq!(result.exit_code, 1);
    let _ = result.duration_ms;
}

#[test]
fn exit_code_reflects_explicit_value() {
    // When: we run a command that exits with a specific code
    let result = run_command("exit 42").expect("spawn should succeed");

    // Then: exit code matches what was requested
    assert_eq!(result.exit_code, 42);
}

#[test]
fn duration_ms_is_plausible() {
    // When: we run a command that sleeps for ~10 ms
    let result = run_command("sleep 0.01").expect("spawn should succeed");

    // Then: measured duration is at least 10 ms
    assert!(result.duration_ms >= 10, "duration_ms = {}", result.duration_ms);
}
