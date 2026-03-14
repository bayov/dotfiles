#!/usr/bin/env python3

import argparse
from concurrent.futures import Future, ThreadPoolExecutor, as_completed
import contextvars
from dataclasses import dataclass
from enum import Enum
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import ClassVar, Iterable, Optional, Sequence, TextIO


# ANSI escape sequences for coloring output.
RESET = "\033[0m"
BOLD = "\033[1m"
RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
CYAN = "\033[36m"
GRAY = "\033[90m"


class LogModule(Enum):
    # Prints a summary of how many files were given as input, and how
    # many were modified.
    SUMMARY = "SUMMARY"

    # Prints each modified file.
    MODIFIED = "MODIFIED"

    # Verbose debug logs.
    DEBUG = "DEBUG"

    # Always printed, indicates an error that occurred while formatting a file.
    ERROR = "ERROR"
    WARN = "WARN"

    @staticmethod
    def cli_controllable() -> list["LogModule"]:
        """
        Log modules that can be specified by CLI. Does not include ERROR and
        WARN since they are always logged."""
        return [
            LogModule.SUMMARY,
            LogModule.MODIFIED,
            LogModule.DEBUG,
        ]


class Logger:
    enabled_log_modules: set[LogModule]
    use_color: bool

    def __init__(self, enabled_log_modules: set[LogModule]):
        self.enabled_log_modules = enabled_log_modules
        self.enabled_log_modules.add(LogModule.ERROR)
        self.enabled_log_modules.add(LogModule.WARN)
        self.use_color = sys.stdout.isatty() and sys.stderr.isatty()

    def log(
        self,
        m: LogModule,
        msg: str,
        file: TextIO = sys.stdout,
        color: Optional[str] = None,
    ):
        if m not in self.enabled_log_modules:
            return

        color = color or self._log_module_color(m)
        reset = RESET if self.use_color else ""

        prefix = f"[{m.value:<{self._max_log_module_len()}}] "
        print(f"{color}{prefix}{msg}{reset}", file=file)

    def summary(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.SUMMARY, msg, color=color)

    def modified(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.MODIFIED, msg, color=color)

    def debug(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.DEBUG, msg, color=color)

    def error(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.ERROR, msg, file=sys.stderr, color=color)

    def warn(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.WARN, msg, file=sys.stderr, color=color)

    @property
    def is_debug_enabled(self) -> bool:
        return LogModule.DEBUG in self.enabled_log_modules

    @property
    def is_summary_enabled(self) -> bool:
        return LogModule.SUMMARY in self.enabled_log_modules

    @property
    def is_modified_enabled(self) -> bool:
        return LogModule.MODIFIED in self.enabled_log_modules

    def color(self, color: str) -> str:
        if not self.use_color:
            return ""
        return color

    def _log_module_color(self, m: LogModule) -> str:
        if not self.use_color:
            return ""
        return {
            LogModule.SUMMARY: GREEN,
            LogModule.MODIFIED: CYAN,
            LogModule.DEBUG: GRAY,
            LogModule.ERROR: BOLD + RED,
            LogModule.WARN: BOLD + YELLOW,
        }[m]

    def _max_log_module_len(self) -> int:
        return max(len(m.value) for m in self.enabled_log_modules)


_logger_ctx: contextvars.ContextVar[Logger] = contextvars.ContextVar("logger")


def logger() -> Logger:
    return _logger_ctx.get()


class FormatterError(Exception):
    pass


class Formatter:
    name: ClassVar[str]
    exts: ClassVar[set[str]]

    # Maximum number of files to format in a single command invocation.
    #
    # Avoid setting too high to prevent hitting command line length limits on
    # some platforms.
    #
    # Use to parallelize formatting of formatters that don't natively run in
    # parallel (e.g. `rustfmt`).
    max_files_per_format: ClassVar[int]

    def format_files(self, files: Sequence[Path]):
        raise NotImplementedError


def _run_formatter_command(
    cmd: list[str], files: Sequence[Path]
) -> subprocess.CompletedProcess:
    files_str = [str(f) for f in files]
    if logger().is_debug_enabled:
        if len(files_str) > 1:
            sep = " \\\n  "
            file_args = sep.join(files_str)
            logger().debug(f"{' '.join(cmd)}{sep}{file_args}")
        else:
            logger().debug(f"{' '.join(cmd + files_str)}")
    try:
        command = cmd + [str(f) for f in files]
        return subprocess.run(
            command, check=True, capture_output=True, text=True
        )
    except subprocess.CalledProcessError as e:
        raise FormatterError(
            f"Failed with exit code {e.returncode}:\n"
            f"Command: {' '.join(cmd)}\n"
            f"Output:\n"
            f"-------\n"
            f"{e.stderr + e.stdout}"
        ) from e


class CmdFormatter(Formatter):
    cmd: ClassVar[list[str]]

    def format_files(self, files: Sequence[Path]):
        _run_formatter_command(self.cmd, files)


class RustfmtFormatter(CmdFormatter):
    name = "rustfmt"
    exts = {".rs"}
    cmd = ["rustfmt"]

    # rustfmt doesn't natively run in parallel.
    max_files_per_format = 20


class DprintFormatter(CmdFormatter):
    name = "dprint"
    exts = {
        ".md",
        ".py",
        ".js",
        ".jsx",
        ".ts",
        ".tsx",
        ".json",
        ".jsonc",
        ".toml",
        ".yaml",
        ".yml",
    }
    cmd = ["dprint", "fmt"]

    # dprint runs in parallel internally, so we can format many files at once.
    max_files_per_format = 1000


class BeancountFormatter(Formatter):
    name = "bean-format"
    exts = {
        ".beancount",
    }

    # bean-format doesn't natively run in parallel, and since we run it without
    # --in-place to avoid mtime updates, we must run it one-by-one.
    max_files_per_format = 1

    def format_files(self, files: Sequence[Path]):
        for f in files:
            content = f.read_text()
            res = _run_formatter_command(cmd=["bean-format"], files=[f])

            formatted = res.stdout
            if formatted != content:
                f.write_text(formatted)


FORMATTERS: Sequence[Formatter] = [
    RustfmtFormatter(),
    DprintFormatter(),
    BeancountFormatter(),
]


def _formatter_by_ext() -> dict[str, Formatter]:
    """
    Raises an error if multiple formatters are associated with the same
    extension.
    """
    result: dict[str, Formatter] = {}
    for formatter in FORMATTERS:
        for ext in formatter.exts:
            if ext in result:
                raise ValueError(
                    f"Extension {repr(ext)} is associated with both "
                    f"{result[ext].name} and {formatter.name}"
                )
            result[ext] = formatter
    return result


formatter_by_ext = _formatter_by_ext()


def jj_repo_root() -> Path:
    try:
        root = subprocess.check_output(
            ["jj", "workspace", "root"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        logger().debug(f"Detected jj repository root: {root}")
        return Path(root)
    except (subprocess.CalledProcessError, FileNotFoundError):
        raise ValueError("Not in a jj repository")


def git_repo_root() -> Path:
    try:
        root = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        logger().debug(f"Detected git repository root: {root}")
        return Path(root)
    except (subprocess.CalledProcessError, FileNotFoundError):
        raise ValueError("Not in a git repository")


def collect_all_tracked_files() -> Iterable[Path]:
    """
    Collects files when `--all-tracked` is specified.

    If we are within a jj repository, we use `jj file list` to collect all
    tracked files with supported extensions.

    Otherwise, if we are within a git repository, we use `git ls-files`.
    """
    files: Iterable[Path]
    try:
        root = jj_repo_root()
        command = ["jj", "file", "list"]
        out = subprocess.check_output(command, cwd=root, text=True)
        logger().debug(
            f"`{' '.join(command)}` returned {len(out.splitlines())} file(s)"
        )
        files = (Path(root) / f for f in out.splitlines())
    except (subprocess.CalledProcessError, FileNotFoundError, ValueError):
        try:
            root = git_repo_root()
            command = ["git", "ls-files"]
            out = subprocess.check_output(command, cwd=root, text=True)
            logger().debug(
                f"`{' '.join(command)}` returned {len(out.splitlines())} file(s)"
            )
            files = (Path(root) / f for f in out.splitlines())
        except (subprocess.CalledProcessError, FileNotFoundError, ValueError):
            raise ValueError(
                "Not in a jj or git repository, cannot use --all-tracked"
            )
    return (f for f in files if f.suffix in formatter_by_ext)


def collect_file_or_directory(path: Path) -> Iterable[Path]:
    """
    Collects files when `--file {path}` is specified.

    If `{path}` is a file, we yield it if it has a supported extension, and
    raise an error otherwise.

    If `{path}` is a directory, we recursively yield all files with supported
    extensions inside it.

    If it's neither, we raise an error.
    """
    if path.is_file():
        if path.suffix in formatter_by_ext:
            yield path
        else:
            raise ValueError(
                f"File {repr(path)} has unsupported extension: "
                f"{repr(path.suffix)}"
            )
    elif path.is_dir():
        for ext in formatter_by_ext.keys():
            for f in path.rglob(f"*{ext}"):
                yield f
    else:
        raise ValueError(f"Path {repr(path)} is not a file or directory")


def collect_git_staged_files() -> Iterable[Path]:
    """
    Collects files when `--git` is specified.

    We use `git diff --cached --name-only` to get the list of staged files,
    and yield those that have supported extensions.
    """
    root = git_repo_root()
    command = ["git", "diff", "--cached", "--name-only"]
    out = subprocess.check_output(command, cwd=root, text=True)
    staged_files = out.splitlines()
    logger().debug(
        f"`{' '.join(command)}` returned {len(staged_files)} file(s)"
    )
    for f in staged_files:
        p = root / f
        if p.suffix in formatter_by_ext:
            yield p


def collect_jj_commit_diff() -> Iterable[Path]:
    """
    Collects files when `--jj` is specified.

    We use `jj diff --name-only` to get the list of changed files in the
    current commit, and yield those that have supported extensions.
    """
    root = jj_repo_root()
    command = ["jj", "diff", "--name-only"]
    out = subprocess.check_output(command, cwd=root, text=True)
    logger().debug(
        f"`{' '.join(command)}` returned {len(out.splitlines())} file(s)"
    )
    for f in out.splitlines():
        p = root / f
        if p.suffix in formatter_by_ext:
            yield p


def collect_files(args) -> Optional[set[Path]]:
    """
    Collects files from all input sources provided in `args` into a single set.

    Returns `None` if no input sources were specified.
    """
    files: set[Path] = set()
    input_sources: list[str] = []
    if args.all_tracked:
        logger().debug("Collecting all tracked files")
        input_sources.append("--all-tracked")
        files.update(collect_all_tracked_files())
    for f in args.file:
        logger().debug(f"Collecting file/directory {f}")
        input_sources.append(f"--file {f}")
        files.update(collect_file_or_directory(Path(f)))
    if args.git:
        logger().debug("Collecting git staged files")
        input_sources.append("--git")
        files.update(collect_git_staged_files())
    if args.jj:
        logger().debug("Collecting jj commit diff files")
        input_sources.append("--jj")
        files.update(collect_jj_commit_diff())
    if not input_sources:
        return None
    return files


def resolve_files(files: set[Path]) -> set[Path]:
    result = set()
    for f in files:
        resolved: Path
        try:
            resolved = f.resolve(strict=True)
        except OSError:
            logger().warn(f"File {repr(f)} cannot be resolved, skipping.")
            continue
        formatter = formatter_by_ext.get(resolved.suffix)
        if formatter is None:
            logger().warn(
                f"File {repr(f)} resolves to {repr(resolved)} with unsupported "
                f"extension {repr(resolved.suffix)}, skipping."
            )
            continue
        if resolved.suffix != f.suffix:
            logger().warn(
                f"File {repr(f)} has extension {repr(f.suffix)}, but resolves "
                f"to {repr(resolved)} with extension {repr(resolved.suffix)}. "
                f"Selected formatter: {formatter.name}."
            )
        result.add(resolved)
    return result


@dataclass(frozen=True)
class FormatResult:
    formatter_name: str
    modified_files: list[Path]
    duration: float
    error: Optional[FormatterError]


def format_files(
    formatter: Formatter,
    chunk: Sequence[Path],
    log: Logger,
) -> FormatResult:
    """
    Formats a chunk of files and returns the result including modified files
    count, the time taken, and the formatter name.
    """
    _logger_ctx.set(log)
    start = time.perf_counter()
    mtimes = {f: os.path.getmtime(f) for f in chunk}
    error: Optional[FormatterError] = None
    try:
        formatter.format_files(chunk)
    except FormatterError as e:
        error = e

    modified_files: list[Path] = []
    for f in chunk:
        if os.path.getmtime(f) != mtimes[f]:
            modified_files.append(f)

    return FormatResult(
        formatter_name=formatter.name,
        modified_files=modified_files,
        duration=time.perf_counter() - start,
        error=error,
    )


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--all-tracked", action="store_true")
    p.add_argument("--file", default=[], action="append")
    p.add_argument("--git", action="store_true")
    p.add_argument("--jj", action="store_true")
    p.add_argument(
        "--log",
        choices=[m.value for m in LogModule.cli_controllable()],
        default=[],
        action="append",
        help="What to log. Can be specified multiple times.",
    )
    args = p.parse_args()

    log_modules = {LogModule(e) for e in args.log}
    # LogModule.DEBUG implies all other log modules:
    if LogModule.DEBUG in log_modules:
        log_modules.update(LogModule.cli_controllable())
    _logger_ctx.set(Logger(log_modules))

    files = collect_files(args)
    if files is None:
        print("No input sources specified.", file=sys.stderr)
        p.print_usage(file=sys.stderr)
        sys.exit(1)

    files = resolve_files(files)

    if not files:
        logger().summary("No files to format.")
        sys.exit(0)

    start_time = time.perf_counter()
    cwd = Path.cwd()
    modified_files_count = 0

    files_by_formatter: dict[Formatter, list[Path]] = {}
    for f in files:
        formatter = formatter_by_ext.get(f.suffix)
        if not formatter:
            # Should not happen since we filter files by supported extensions
            # when collecting.
            raise ValueError(
                f"No formatter found for file {repr(f)} with extension "
                f"{repr(f.suffix)}"
            )
        files_by_formatter.setdefault(formatter, []).append(f)

    formatter_durations: dict[str, float] = {}

    with ThreadPoolExecutor() as executor:
        futures: list[Future[FormatResult]] = []
        for formatter, group_files in files_by_formatter.items():
            for i in range(0, len(group_files), formatter.max_files_per_format):
                chunk = group_files[i : i + formatter.max_files_per_format]
                futures.append(
                    executor.submit(
                        format_files,
                        formatter,
                        chunk,
                        logger(),
                    )
                )

        for future in as_completed(futures):
            res = future.result()
            for f in res.modified_files:
                relative_to_cwd = f.relative_to(cwd)
                logger().modified(
                    f"{logger().color(GRAY)}"
                    f"({res.formatter_name}) "
                    f"{logger().color(RESET)}"
                    f"{relative_to_cwd}"
                )
            if res.error:
                logger().error(str(res.error))
                sys.exit(1)
            modified_files_count += len(res.modified_files)
            formatter_durations[res.formatter_name] = (
                formatter_durations.get(res.formatter_name, 0.0) + res.duration
            )

    total_duration = time.perf_counter() - start_time
    logger().summary(
        f"Modified {modified_files_count} out of {len(files)} file(s) in "
        f"{total_duration:.2f}s.",
        # Change summary color to be GRAY if no files have been modified, for
        # a less distracting output.
        color=GRAY if modified_files_count == 0 else None,
    )

    if logger().is_debug_enabled:
        for name, duration in formatter_durations.items():
            logger().debug(f"Time spent in {name}: {duration:.2f}s")


if __name__ == "__main__":
    main()
