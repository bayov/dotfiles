#!/usr/bin/env python3

import argparse
import os
import pty
import re
import select
import shlex
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path

from .util import (
    BOLD,
    CYAN,
    GRAY,
    GREEN,
    RED,
    RESET,
    LogModule,
    Logger,
    logger,
    require_project_root,
    set_logger,
)

CONFIG_PATH = Path(".config/test.toml")

_ANSI_ESCAPE = re.compile(rb"\x1b(?:[@-Z\\-_]|\[[0-9;]*[ -/]*[@-~])")


def _strip_ansi(data: bytes) -> bytes:
    return _ANSI_ESCAPE.sub(b"", data)


def load_test_suites() -> list[dict]:
    """Loads test commands from `.config/test.toml` in the current directory.

    The config file must define a `[[test_suite]]` array where each entry has:
    - `name`: display name for the command
    - `command`: shell command string to execute
    - `cwd` (optional): working directory relative to the project root
    """
    if not CONFIG_PATH.exists():
        print(
            f"error: config file not found: {CONFIG_PATH}",
            file=sys.stderr,
        )
        sys.exit(1)
    with open(CONFIG_PATH, "rb") as f:
        config = tomllib.load(f)
    return config.get("test_suite", [])


def draw_box(text: str) -> str:
    lines = text.splitlines()
    width = max(len(line) for line in lines) + 2
    res = []
    res.append("┏" + "━" * width + "┓")
    for line in lines:
        res.append(f"┃ {line.ljust(width - 2)} ┃")
    res.append("┗" + "━" * width + "┛")
    return "\n".join(res)


def run_command(
    cmd_args: list[str],
    log_file_path: str | None,
    quiet: bool,
    cwd: str | None = None,
    capture: bool = False,
) -> subprocess.CompletedProcess:
    """Runs a command, teeing output to the log file and optionally stdout.

    If not quiet and stdout is a TTY, uses a pseudo-terminal to preserve colors.
    `cwd` sets the working directory for the subprocess; defaults to the current
    directory if not specified.
    If `capture` is True, the ANSI-stripped output is returned in `.stdout`.
    """
    master_fd, slave_fd = pty.openpty()
    process = subprocess.Popen(
        cmd_args,
        stdin=subprocess.DEVNULL,
        stdout=slave_fd,
        stderr=slave_fd,
        close_fds=True,
        cwd=cwd,
    )
    os.close(slave_fd)
    read_fd = master_fd

    log_file = open(log_file_path, "ab") if log_file_path else None
    captured = bytearray() if capture else None

    try:
        while True:
            # Use select with a short timeout so we can detect when the
            # main process exits even if child processes (e.g. TypeScript
            # language server workers) still hold the PTY slave fd open.
            r, _, _ = select.select([read_fd], [], [], 0.05)
            if not r:
                if process.poll() is not None:
                    break
                continue

            try:
                chunk = os.read(read_fd, 1024)
            except OSError:
                # This happens when the slave_fd is closed (process finished)
                break
            if not chunk:
                break

            stripped = _strip_ansi(chunk)
            if log_file:
                log_file.write(stripped)
                log_file.flush()
            if captured is not None:
                captured.extend(stripped)

            if not quiet:
                sys.stdout.buffer.write(chunk)
                sys.stdout.buffer.flush()
    finally:
        if log_file:
            log_file.close()
        os.close(read_fd)

    returncode = process.wait()
    return subprocess.CompletedProcess(
        cmd_args,
        returncode,
        stdout=bytes(captured) if captured is not None else None,
    )


def main():
    require_project_root()

    parser = argparse.ArgumentParser(description="Run project test suite.")
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose debug logging",
    )
    parser.add_argument(
        "--quiet",
        "-q",
        action="store_true",
        help="Silence test command output (still stored in /tmp)",
    )
    parser.add_argument(
        "--sequential",
        "-s",
        action="store_true",
        help="Run tests sequentially instead of in parallel",
    )
    parser.add_argument(
        "--show-failures",
        "-f",
        action="store_true",
        help="When used with --quiet, print output of failed tests",
    )
    args = parser.parse_args()

    enabled_modules = {LogModule.INFO}
    if args.verbose:
        enabled_modules.add(LogModule.DEBUG)

    set_logger(Logger(enabled_modules, show_timestamps=True))

    test_suites = load_test_suites()

    results: list[tuple[str, int, float]] = []

    suite_start_time = time.time()

    log_file_path = None
    if args.quiet or not args.sequential:
        fd, log_file_path = tempfile.mkstemp(
            prefix="test_suite_", suffix=".log"
        )
        os.close(fd)
        logger().info(
            f"Test command output will be directed to: {log_file_path}"
        )

    if args.sequential:
        for test_suite in test_suites:
            name = test_suite["name"]
            command_str = test_suite["command"]
            cmd_args = shlex.split(command_str)

            if log_file_path:
                header = draw_box(f"{command_str}")
                with open(log_file_path, "a") as f:
                    if test_suite != test_suites[0]:
                        f.write("\n\n")
                    f.write(f"{header}\n")

            logger().info(f"Running {name}...")
            if logger().is_debug_enabled:
                logger().debug(f"Command: {command_str}")

            capture = args.quiet and args.show_failures
            start_time = time.time()
            res = run_command(
                cmd_args,
                log_file_path,
                args.quiet,
                test_suite.get("cwd"),
                capture=capture,
            )
            duration = time.time() - start_time

            if capture and res.returncode != 0 and res.stdout:
                indented = b"".join(
                    b"    " + line
                    for line in res.stdout.strip().splitlines(keepends=True)
                )
                print()
                logger().info(f"{name} output:")
                print()
                sys.stdout.buffer.write(indented)
                print("\n")
                sys.stdout.buffer.flush()

            results.append((name, res.returncode, duration))

            if res.returncode == 0:
                logger().info(
                    f"✓ {name} passed {GRAY}({duration:.2f}s){RESET}",
                    color=GREEN,
                )
            else:
                logger().error(
                    f"✗ {name} failed {GRAY}({duration:.2f}s){RESET}"
                )
    else:
        processes = []
        for test_suite in test_suites:
            name = test_suite["name"]
            command_str = test_suite["command"]
            cmd_args = shlex.split(command_str)

            logger().info(f"Running {name}...")
            # Create a separate log file for each command to avoid interleaving
            fd, cmd_log_path = tempfile.mkstemp(
                prefix=f"test_{name.replace(' ', '_')}_", suffix=".log"
            )
            os.close(fd)

            # Run in background using a pipe (no PTY, suppresses progress bars).
            # Force colors via env vars since there's no TTY.
            cwd = test_suite.get("cwd")
            start_time = time.time()
            env = os.environ.copy()
            env["FORCE_COLOR"] = "1"
            env["CLICOLOR_FORCE"] = "1"
            proc = subprocess.Popen(
                cmd_args,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                close_fds=True,
                cwd=cwd,
                env=env,
            )
            read_fd = proc.stdout.fileno()

            processes.append(
                {
                    "name": name,
                    "proc": proc,
                    "start_time": start_time,
                    "log_file": open(cmd_log_path, "wb"),
                    "log_path": cmd_log_path,
                    "command_str": command_str,
                    "duration": None,
                    "returncode": None,
                    "read_fd": read_fd,
                    "eof": False,
                }
            )

        while any(not p["eof"] for p in processes):
            read_fds = [p["read_fd"] for p in processes if not p["eof"]]
            r, _, _ = select.select(read_fds, [], [], 0.1)

            for fd in r:
                p = next(p for p in processes if p["read_fd"] == fd)
                try:
                    chunk = os.read(fd, 4096)
                    if chunk:
                        p["log_file"].write(chunk)
                    else:
                        p["eof"] = True
                except OSError:
                    # Happens when the slave is closed
                    p["eof"] = True

            for p in processes:
                if p["returncode"] is None:
                    code = p["proc"].poll()
                    if code is not None:
                        p["returncode"] = code
                        p["duration"] = time.time() - p["start_time"]

                        name = p["name"]
                        duration = p["duration"]
                        results.append((name, code, duration))

                        # Mark EOF before closing so this fd is excluded from
                        # future select() calls. Child processes that still hold
                        # the PTY slave fd open (e.g. TypeScript workers) would
                        # otherwise block the loop forever.
                        p["eof"] = True
                        p["log_file"].close()
                        p["proc"].stdout.close()

                        # Read output from individual log
                        with open(p["log_path"], "rb") as f_cmd:
                            output = f_cmd.read()

                        header = draw_box(f"{p['command_str']}").encode()

                        indented_output = b""
                        if output and (
                            not args.quiet or (args.show_failures and code != 0)
                        ):
                            indented_output = b"".join(
                                b"    " + line
                                for line in output.strip().splitlines(
                                    keepends=True
                                )
                            )

                        if indented_output:
                            print()
                            logger().info(f"{name} output:")
                            print()
                            sys.stdout.buffer.write(indented_output)
                            print("\n")
                            sys.stdout.buffer.flush()

                        # Append individual log to main log file (strip ANSI for clean text)
                        if log_file_path:
                            with open(log_file_path, "ab") as f_main:
                                f_main.write(b"\n\n" + header + b"\n")
                                f_main.write(_strip_ansi(output))

                        if code == 0:
                            logger().info(
                                f"✓ {name} passed {GRAY}({duration:.2f}s){RESET}",
                                color=GREEN,
                            )
                        else:
                            logger().error(
                                f"✗ {name} failed {GRAY}({duration:.2f}s){RESET}"
                            )

                        os.remove(p["log_path"])

    # Summary
    total_duration = time.time() - suite_start_time
    max_name_len = max(len(name) for name, _, _ in results) if results else 0
    padding = max_name_len + 2

    print()
    logger().info("Summary:", color=CYAN)
    any_failed = False
    for name, code, duration in results:
        duration_str = f"{GRAY}({duration:.2f}s){RESET}"
        if code == 0:
            logger().info(
                f"  ✓ {name:<{padding}} PASSED {duration_str}", color=GREEN
            )
        else:
            logger().info(
                f"  ✗ {name:<{padding}} FAILED {duration_str}", color=RED
            )
            any_failed = True

    duration_suffix = f" {GRAY}({total_duration:.2f}s){RESET}"
    suite_padding = max(0, padding - 6)
    if any_failed:
        logger().error(
            f"Test suite {'':{suite_padding}}FAILED{duration_suffix}",
            color=BOLD + RED,
        )
    else:
        logger().info(
            f"Test suite {'':{suite_padding}}PASSED{duration_suffix}",
            color=BOLD + GREEN,
        )

    if log_file_path:
        logger().info(f"Output log: {log_file_path}", color=GRAY)

    sys.exit(1 if any_failed else 0)


if __name__ == "__main__":
    main()
