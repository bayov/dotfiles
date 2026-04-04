//! Shell command execution for workflow command steps.
//!
//! The single public entry point is [`run_command`], which spawns a shell
//! command via `sh -c` and returns the exit code and elapsed time.

use std::time::Instant;

use tracing::instrument;

use crate::result::Result;

// ---------------------------------------------------------------------------
// CommandResult
// ---------------------------------------------------------------------------

/// The outcome of a shell command execution.
pub struct CommandResult {
    /// The exit code returned by the process.
    ///
    /// Conventionally `0` means success and any non-zero value means failure,
    /// though the exact meaning is command-dependent.
    pub exit_code: i32,

    /// Wall-clock time from spawn to exit, in milliseconds.
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Spawns `sh -c <shell_cmd>`, waits for it to finish, and returns the result.
///
/// The child process **inherits** the caller's stdin, stdout, and stderr — they
/// are passed through directly to the terminal and are not captured. This is
/// intentional for v1: command output is visible to the user in real time.
///
/// The returned [`CommandResult`] contains:
/// - `exit_code`: the process exit code (e.g. `0` for success, `1` for
///   failure). On Unix, if the process was killed by a signal, the exit code
///   defaults to `-1` because [`std::process::ExitStatus::code`] returns
///   `None`.
/// - `duration_ms`: elapsed wall-clock time from spawn to exit.
///
/// # Errors
///
/// Returns [`crate::Error::Io`] if the `sh` process could not be spawned (e.g.
/// `sh` is not found on `PATH` or the caller lacks execute permissions).
#[instrument(fields(cmd = shell_cmd))]
pub fn run_command(shell_cmd: &str) -> Result<CommandResult> {
    let start = Instant::now();

    // Spawn `sh -c <shell_cmd>` with inherited stdio so output flows to the
    // terminal rather than being buffered or discarded.
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(shell_cmd)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let status = child.wait()?;
    let duration_ms =
        u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

    // `code()` returns `None` on Unix when the process was killed by a signal;
    // we map that to -1 as a sentinel.
    let exit_code = status.code().unwrap_or(-1);

    Ok(CommandResult { exit_code, duration_ms })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "command.tests.rs"]
mod tests;
