#!/usr/bin/env python3

import argparse
from concurrent.futures import Future, ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
import fnmatch
import os
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import Iterable, Optional, Sequence

from .util import (
    GRAY,
    GREEN,
    LogModule,
    Logger,
    git_repo_root,
    jj_repo_root,
    logger,
    project_root,
    require_project_root,
    set_logger,
)


class FormatterError(Exception):
    pass


@dataclass(eq=False)
class Formatter:
    name: str
    exts: set[str]

    # Maximum number of files to format in a single command invocation.
    #
    # Avoid setting too high to prevent hitting command line length limits on
    # some platforms.
    #
    # Use to parallelize formatting of formatters that don't natively run in
    # parallel (e.g. `rustfmt`).
    max_files_per_format: int

    def format_files(self, files: Sequence[Path]):
        """Format the given files in place. Must be implemented by subclasses."""
        raise NotImplementedError


def _run_formatter_command(
    cmd: list[str],
    files: Sequence[Path],
    cwd: Optional[Path] = None,
) -> subprocess.CompletedProcess:
    """Run a formatter command with the given files appended as arguments.

    Logs the command at DEBUG level. Raises FormatterError on non-zero exit.
    """
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
            command,
            check=True,
            capture_output=True,
            text=True,
            cwd=cwd,
            stdin=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError as e:
        raise FormatterError(
            f"Failed with exit code {e.returncode}:\n"
            f"Command: {' '.join(cmd)}\n"
            f"Output:\n"
            f"-------\n"
            f"{e.stderr + e.stdout}"
        ) from e


@dataclass(eq=False)
class CmdFormatter(Formatter):
    cmd: list[str] = field(default_factory=list)

    # Working directory for the subprocess. Supports `{project_root}` placeholder.
    cwd: Optional[Path] = None

    # When True, the command outputs the formatted content to stdout rather than
    # modifying the file in place. The script writes back to the file only if
    # the content differs from the original, to avoid spurious mtime updates.
    #
    # Required for formatters like `bean-format` that do not support `--in-place`.
    stdout_mode: bool = False

    def format_files(self, files: Sequence[Path]):
        """Format files using the configured command.

        Resolves `{project_root}` placeholders in `cmd` and `cwd`. In
        `stdout_mode`, runs the command per-file and writes back only if the
        output differs from the original content.
        """
        resolved_cmd = [
            arg.replace("{project_root}", str(project_root()))
            for arg in self.cmd
        ]
        resolved_cwd = (
            Path(str(self.cwd).replace("{project_root}", str(project_root())))
            if self.cwd is not None
            else None
        )

        if self.stdout_mode:
            for f in files:
                content = f.read_text()
                res = _run_formatter_command(
                    resolved_cmd, [f], cwd=resolved_cwd
                )
                if res.stdout != content:
                    f.write_text(res.stdout)
        else:
            _run_formatter_command(resolved_cmd, files, cwd=resolved_cwd)


def load_formatters(config_path: Optional[Path] = None) -> list[CmdFormatter]:
    """Loads formatter definitions from a TOML config file.

    Reads `[[formatter]]` entries and constructs `CmdFormatter` instances.
    Each entry must have `name`, `exts`, `cmd`, and `max_files_per_format`.
    Optional fields: `cwd` (string path, supports `{project_root}`),
    `stdout_mode` (bool, default false).

    Exits with an error if the config file is not found or is malformed.
    """
    if config_path is None:
        config_path = project_root() / ".config" / "format.toml"
    if not config_path.exists():
        print(
            f"error: formatter config not found: {config_path}\n\n"
            f"Create .config/format.toml in your project root.\n"
            f"See examples/format.toml in the devscripts repo for the schema.",
            file=sys.stderr,
        )
        sys.exit(1)
    with open(config_path, "rb") as f:
        data = tomllib.load(f)
    formatters = []
    for i, entry in enumerate(data.get("formatter", [])):
        for required in ("name", "exts", "cmd", "max_files_per_format"):
            if required not in entry:
                print(
                    f"error: formatter[{i}] missing required field '{required}'",
                    file=sys.stderr,
                )
                sys.exit(1)
        formatters.append(
            CmdFormatter(
                name=entry["name"],
                exts=set(entry["exts"]),
                cmd=entry["cmd"],
                max_files_per_format=entry["max_files_per_format"],
                cwd=Path(entry["cwd"]) if "cwd" in entry else None,
                stdout_mode=entry.get("stdout_mode", False),
            )
        )
    return formatters


FORMATTERS: list[CmdFormatter] = load_formatters()


def _all_supported_exts() -> set[str]:
    """Return the union of all file extensions claimed by any formatter."""
    return {ext for fmt in FORMATTERS for ext in fmt.exts}


# All file extensions handled by at least one formatter. Used to pre-filter
# files before path-aware routing via `find_formatter`.
supported_exts = _all_supported_exts()


def find_formatter(file: Path) -> Optional[CmdFormatter]:
    """Returns the formatter for `file`, or `None` if none applies."""
    if file.suffix not in supported_exts:
        return None
    for fmt in FORMATTERS:
        if file.suffix in fmt.exts:
            return fmt
    return None


def _validate_formatters() -> None:
    """Ensures no two formatters claim the same extension."""
    for ext in supported_exts:
        handlers = [f for f in FORMATTERS if ext in f.exts]
        if len(handlers) > 1:
            names = ", ".join(f.name for f in handlers)
            raise ValueError(
                f"Extension {ext!r} is claimed by multiple formatters: {names}"
            )


_validate_formatters()


def _gitattributes_binary_patterns() -> list[str]:
    """Returns glob patterns marked as `binary` in the project's .gitattributes.

    Only patterns with the `binary` attribute (a shorthand for
    `-text -diff merge=binary`) are returned. Patterns without this attribute
    are ignored.
    """
    path = project_root() / ".gitattributes"
    if not path.exists():
        return []
    patterns = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) >= 2 and "binary" in parts[1:]:
            patterns.append(parts[0])
    return patterns


def _is_binary_file(file: Path, binary_patterns: list[str]) -> bool:
    """Returns True if `file` matches any binary pattern from .gitattributes.

    Matches are attempted against both the filename and the path relative to
    `project_root()`, so patterns like `uv.lock` and `docs/*.pdf` both work.
    """
    try:
        rel = str(file.relative_to(project_root()))
    except ValueError:
        rel = file.name
    return any(
        fnmatch.fnmatch(file.name, p) or fnmatch.fnmatch(rel, p)
        for p in binary_patterns
    )


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
    return (f for f in files if f.suffix in supported_exts)


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
        if path.suffix in supported_exts:
            yield path
        else:
            raise ValueError(
                f"File {repr(path)} has unsupported extension: "
                f"{repr(path.suffix)}"
            )
    elif path.is_dir():
        for ext in supported_exts:
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
        if p.suffix in supported_exts:
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
        if p.suffix in supported_exts:
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

    # Exclude files marked as binary in .gitattributes — they should not be
    # reformatted as text.
    binary_patterns = _gitattributes_binary_patterns()
    if binary_patterns:
        filtered: set[Path] = set()
        for f in files:
            if _is_binary_file(f, binary_patterns):
                logger().debug(f"Skipping binary file: {f}")
            else:
                filtered.add(f)
        files = filtered

    return files


def resolve_files(files: set[Path]) -> set[Path]:
    """Resolve each path to its canonical form, filtering out unusable entries.

    Skips paths that cannot be resolved (e.g. broken symlinks) or that have
    no applicable formatter after resolution. Warns on either condition, and
    also warns when a symlink's extension differs from the resolved file's.
    """
    result = set()
    for f in files:
        resolved: Path
        try:
            resolved = f.resolve(strict=True)
        except OSError:
            logger().warn(f"File {repr(f)} cannot be resolved, skipping.")
            continue
        formatter = find_formatter(resolved)
        if formatter is None:
            logger().warn(
                f"File {repr(f)} resolves to {repr(resolved)}: "
                f"no formatter applies, skipping."
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
    # Absolute perf_counter timestamps for this chunk, used to compute
    # wall-clock time per formatter across parallel chunks.
    start_time: float
    end_time: float
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
    set_logger(log)
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
        start_time=start,
        end_time=time.perf_counter(),
        error=error,
    )


def main():
    """Entry point. Parses arguments, collects files, and runs formatters."""
    require_project_root()

    p = argparse.ArgumentParser()
    p.add_argument("--all-tracked", action="store_true")
    p.add_argument("--file", default=[], action="append")
    p.add_argument("--git", action="store_true")
    p.add_argument("--jj", action="store_true")
    p.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable verbose debug logging.",
    )
    args = p.parse_args()

    log_modules = {LogModule.INFO}
    if args.verbose:
        log_modules.add(LogModule.DEBUG)
    set_logger(Logger(log_modules))

    files = collect_files(args)
    if files is None:
        print("No input sources specified.", file=sys.stderr)
        p.print_usage(file=sys.stderr)
        sys.exit(1)

    files = resolve_files(files)

    if not files:
        logger().info("No files to format.", color=GREEN)
        sys.exit(0)

    start_time = time.perf_counter()
    cwd = Path.cwd()
    modified_files_count = 0

    files_by_formatter: dict[Formatter, list[Path]] = {}
    for f in files:
        formatter = find_formatter(f)
        if not formatter:
            # Should not happen since resolve_files already skips files with
            # no applicable formatter.
            raise ValueError(
                f"No formatter found for file {repr(f)} with extension "
                f"{repr(f.suffix)}"
            )
        files_by_formatter.setdefault(formatter, []).append(f)

    # Per-formatter (min_start, max_end) across parallel chunks, so we can
    # report actual wall-clock time rather than a misleading cumulative sum.
    formatter_spans: dict[str, tuple[float, float]] = {}

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
                logger().info(
                    f"({res.formatter_name}) Formatted: {relative_to_cwd}"
                )
            if res.error:
                logger().error(str(res.error))
                sys.exit(1)
            modified_files_count += len(res.modified_files)
            prev = formatter_spans.get(res.formatter_name)
            if prev is None:
                formatter_spans[res.formatter_name] = (
                    res.start_time,
                    res.end_time,
                )
            else:
                formatter_spans[res.formatter_name] = (
                    min(prev[0], res.start_time),
                    max(prev[1], res.end_time),
                )

    total_duration = time.perf_counter() - start_time
    logger().info(
        f"Modified {modified_files_count} out of {len(files)} file(s) in "
        f"{total_duration:.2f}s.",
        # Change color to be GRAY if no files have been modified, for
        # a less distracting output.
        color=GRAY if modified_files_count == 0 else GREEN,
    )

    if logger().is_debug_enabled:
        for name, (start, end) in formatter_spans.items():
            logger().debug(f"Time spent in {name}: {end - start:.2f}s")


if __name__ == "__main__":
    main()
