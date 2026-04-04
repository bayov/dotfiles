#!/usr/bin/env python3
"""Format source files using per-extension formatter commands."""

import argparse
import fnmatch
import os
import subprocess
import sys
import time
import tomllib
from collections.abc import Iterable, Sequence
from concurrent.futures import Future, ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path
from typing import Protocol, cast, override

from .util import (
    GRAY,
    GREEN,
    RESET,
    YELLOW,
    Logger,
    LogModule,
    git_repo_root,
    jj_repo_root,
    logger,
    project_root,
    require_project_root,
    set_logger,
)


class FormatterError(Exception):
    """Raised when a formatter command fails."""


@dataclass(eq=False)
class Formatter:
    """Base class for file formatters."""

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

    # Interpreter names (e.g. "python3", "bash") matched against the shebang
    # of extensionless files. Extracted from lines like `#!/usr/bin/env python3`
    # or `#!/usr/bin/python3` by taking the last path component after stripping
    # `env`.
    shebangs: set[str] = field(default_factory=set[str])

    # Glob patterns (e.g. "**/Makefile", "**/.gitignore") matched against
    # the file path. Takes priority over extension and shebang matching.
    globs: set[str] = field(default_factory=set[str])

    def format_files(self, files: Sequence[Path]) -> None:
        """
        Format the given files in place.

        Must be implemented by subclasses.
        """
        raise NotImplementedError


def _run_formatter_command(
    cmd: list[str],
    files: Sequence[Path],
    cwd: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    """
    Run a formatter command with the given files appended as arguments.

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
    """Formatter that runs an external command."""

    cmd: list[str] = field(default_factory=list[str])

    # Working directory for the subprocess. Supports `{project_root}`
    # placeholder.
    cwd: Path | None = None

    # When True, the command outputs the formatted content to stdout rather than
    # modifying the file in place. The script writes back to the file only if
    # the content differs from the original, to avoid spurious mtime updates.
    #
    # Required for formatters like `bean-format` that do not support
    # `--in-place`.
    stdout_mode: bool = False

    @override
    def format_files(self, files: Sequence[Path]) -> None:
        """
        Format files using the configured command.

        Resolve `{project_root}` placeholders in `cmd` and `cwd`. In
        `stdout_mode`, run the command per-file and write back only if the
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


def load_formatters(config_path: Path | None = None) -> list[CmdFormatter]:
    """
    Load formatter definitions from a TOML config file.

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
            f"{YELLOW}warning: formatter config not found:"
            f" {config_path}, skipping.{RESET}",
            file=sys.stderr,
        )
        sys.exit(0)
    with open(config_path, "rb") as f:
        data = tomllib.load(f)
    formatters: list[CmdFormatter] = []
    for i, entry in enumerate(data.get("formatter", [])):
        for required in ("name", "exts", "cmd", "max_files_per_format"):
            if required not in entry:
                print(
                    f"error: formatter[{i}] missing required"
                    f" field '{required}'",
                    file=sys.stderr,
                )
                sys.exit(1)
        formatters.append(
            CmdFormatter(
                name=entry["name"],
                exts=set(entry["exts"]),
                max_files_per_format=entry["max_files_per_format"],
                shebangs=set(entry.get("shebangs", [])),
                globs=set(entry.get("globs", [])),
                cmd=entry["cmd"],
                cwd=Path(entry["cwd"]) if "cwd" in entry else None,
                stdout_mode=entry.get("stdout_mode", False),
            )
        )
    return formatters


@dataclass(frozen=True)
class IgnoreConfig:
    """Ignore patterns that exclude files from formatter warnings."""

    exts: set[str]
    shebangs: set[str]
    globs: set[str]

    def matches(self, file: Path) -> bool:
        """Return True if `file` matches any ignore pattern."""
        return _file_matches(file, self.exts, self.shebangs, self.globs)


def load_ignore(config_path: Path | None = None) -> IgnoreConfig:
    """
    Load the `[ignore]` section from the formatter config file.

    Optional fields: `exts` (list of extensions, e.g. `[".lock"]`),
    `shebangs` (list of interpreter names, e.g. `["bash"]`),
    `globs` (list of glob patterns, e.g. `["**/.gitignore"]`).
    Files matching any will be silently excluded from the no-formatter warning.
    Returns an empty `IgnoreConfig` if the section is absent.
    """
    if config_path is None:
        config_path = project_root() / ".config" / "format.toml"
    if not config_path.exists():
        return IgnoreConfig(exts=set(), shebangs=set(), globs=set())
    with open(config_path, "rb") as f:
        data = tomllib.load(f)
    entry = data.get("ignore", {})
    return IgnoreConfig(
        exts=set(entry.get("exts", [])),
        shebangs=set(entry.get("shebangs", [])),
        globs=set(entry.get("globs", [])),
    )


@dataclass(frozen=True)
class _FormatRegistry:
    formatters: list[CmdFormatter]
    ignore: IgnoreConfig
    supported_exts: frozenset[str]
    supported_globs: frozenset[str]


@lru_cache(maxsize=1)
def _registry() -> _FormatRegistry:
    formatters = load_formatters()
    ignore = load_ignore()
    exts = frozenset(ext for fmt in formatters for ext in fmt.exts)
    globs = frozenset(g for fmt in formatters for g in fmt.globs)
    _validate_formatters(formatters, exts, globs)
    return _FormatRegistry(formatters, ignore, exts, globs)


def _read_shebang(file: Path) -> str | None:
    """Return the raw shebang line (e.g. `#!/usr/bin/env python3`), or None."""
    try:
        with open(file, "rb") as f:
            first_line = (
                f.readline(512).decode("utf-8", errors="replace").strip()
            )
    except OSError:
        return None
    return first_line if first_line.startswith("#!") else None


def _shebang_interpreter(file: Path) -> str | None:
    """
    Return the interpreter name from the shebang, or None.

    Handles both `#!/usr/bin/env python3` (returns `python3`) and
    `#!/usr/bin/python3` (returns `python3`).
    """
    shebang = _read_shebang(file)
    if shebang is None:
        return None
    parts = shebang[2:].split()
    if not parts:
        return None
    if Path(parts[0]).name == "env":
        return parts[1] if len(parts) > 1 else None
    return Path(parts[0]).name


def _file_matches(
    file: Path,
    exts: set[str],
    shebangs: set[str],
    globs: set[str],
) -> bool:
    """
    Return True if `file` matches any of the given patterns.

    Glob patterns take priority and match against the full file path.
    Extension patterns only apply to files that have an extension.
    Shebang patterns only apply to extensionless files (including dotfiles
    like `.gitignore`, where `Path(".gitignore").suffix == ""`).
    """
    if any(file.match(g) for g in globs):
        return True
    if file.suffix:
        return file.suffix in exts
    interpreter = _shebang_interpreter(file)
    return interpreter is not None and interpreter in shebangs


def find_formatter(file: Path) -> CmdFormatter | None:
    """Return the formatter for `file`, or `None` if none applies."""
    for fmt in _registry().formatters:
        if _file_matches(file, fmt.exts, fmt.shebangs, fmt.globs):
            return fmt
    return None


def _validate_formatters(
    formatters: list[CmdFormatter],
    supported_exts: frozenset[str],
    supported_globs: frozenset[str],
) -> None:
    """Ensure no two formatters claim the same extension, shebang, or glob."""
    for ext in supported_exts:
        handlers = [f for f in formatters if ext in f.exts]
        if len(handlers) > 1:
            names = ", ".join(f.name for f in handlers)
            raise ValueError(
                f"Extension {ext!r} is claimed by multiple formatters: {names}"
            )
    all_shebangs = [s for fmt in formatters for s in fmt.shebangs]
    for shebang in all_shebangs:
        handlers = [f for f in formatters if shebang in f.shebangs]
        if len(handlers) > 1:
            names = ", ".join(f.name for f in handlers)
            raise ValueError(
                f"Shebang {shebang!r} is claimed by multiple"
                f" formatters: {names}"
            )
    for glob in supported_globs:
        handlers = [f for f in formatters if glob in f.globs]
        if len(handlers) > 1:
            names = ", ".join(f.name for f in handlers)
            raise ValueError(
                f"Glob {glob!r} is claimed by multiple formatters: {names}"
            )


def _gitattributes_binary_patterns() -> list[str]:
    """
    Return glob patterns marked as `binary` in the project's .gitattributes.

    Only patterns with the `binary` attribute (a shorthand for
    `-text -diff merge=binary`) are returned. Patterns without this attribute
    are ignored.
    """
    path = project_root() / ".gitattributes"
    if not path.exists():
        return []
    patterns: list[str] = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) >= 2 and "binary" in parts[1:]:
            patterns.append(parts[0])
    return patterns


def _is_binary_file(file: Path, binary_patterns: list[str]) -> bool:
    """
    Return True if `file` matches any binary pattern from .gitattributes.

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
    Collect files when `--all-tracked` is specified.

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
                f"`{' '.join(command)}` returned"
                f" {len(out.splitlines())} file(s)"
            )
            files = (Path(root) / f for f in out.splitlines())
        except (
            subprocess.CalledProcessError,
            FileNotFoundError,
            ValueError,
        ) as err:
            raise ValueError(
                "Not in a jj or git repository, cannot use --all-tracked"
            ) from err
    return files


def collect_file_or_directory(path: Path) -> Iterable[Path]:
    """
    Collect files when `--file {path}` is specified.

    If `{path}` is a file, we yield it if it has a supported extension, and
    raise an error otherwise.

    If `{path}` is a directory, we recursively yield all files with supported
    extensions inside it.

    If it's neither, we raise an error.
    """
    reg = _registry()
    if path.is_file():
        if (
            path.suffix in reg.supported_exts
            or path.suffix == ""
            or any(path.match(g) for g in reg.supported_globs)
        ):
            yield path
        else:
            raise ValueError(
                f"File {path!r} has unsupported extension: {path.suffix!r}"
            )
    elif path.is_dir():
        for ext in reg.supported_exts:
            for f in path.rglob(f"*{ext}"):
                yield f
        for f in path.rglob("*"):
            if f.is_file() and (
                f.suffix == "" or any(f.match(g) for g in reg.supported_globs)
            ):
                yield f
    else:
        raise ValueError(f"Path {path!r} is not a file or directory")


def collect_git_staged_files() -> Iterable[Path]:
    """
    Collect files when `--git` is specified.

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
        yield root / f


def collect_jj_commit_diff() -> Iterable[Path]:
    """
    Collect files when `--jj` is specified.

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
        yield root / f


class _CollectArgs(Protocol):
    all_tracked: bool
    file: list[str]
    git: bool
    jj: bool


def collect_files(args: _CollectArgs) -> tuple[set[Path], list[Path]] | None:
    """
    Collect files from all input sources provided in `args` into a single set.

    Returns `None` if no input sources were specified. Otherwise returns a tuple
    of (formattable files, files skipped due to unsupported extension).
    """
    all_files: set[Path] = set()
    input_sources: list[str] = []
    if args.all_tracked:
        logger().debug("Collecting all tracked files")
        input_sources.append("--all-tracked")
        all_files.update(collect_all_tracked_files())
    for f in args.file:
        logger().debug(f"Collecting file/directory {f}")
        input_sources.append(f"--file {f}")
        all_files.update(collect_file_or_directory(Path(f)))
    if args.git:
        logger().debug("Collecting git staged files")
        input_sources.append("--git")
        all_files.update(collect_git_staged_files())
    if args.jj:
        logger().debug("Collecting jj commit diff files")
        input_sources.append("--jj")
        all_files.update(collect_jj_commit_diff())
    if not input_sources:
        return None

    # Split into formattable files and those with an unsupported extension.
    # Extensionless files and exact-filename matches are always passed through;
    # shebang matching happens later in find_formatter / resolve_files.
    reg = _registry()
    files: set[Path] = set()
    unsupported_ext_files: list[Path] = []
    for f in all_files:
        if (
            f.suffix in reg.supported_exts
            or f.suffix == ""
            or any(f.match(g) for g in reg.supported_globs)
        ):
            files.add(f)
        else:
            unsupported_ext_files.append(f)

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

    return files, unsupported_ext_files


def resolve_files(files: set[Path]) -> tuple[set[Path], list[Path]]:
    """
    Resolve each path to its canonical form, filtering out unusable entries.

    Skips paths that cannot be resolved (e.g. broken symlinks) or that have
    no applicable formatter after resolution. Warns on either condition, and
    also warns when a symlink's extension differs from the resolved file's.
    """
    result: set[Path] = set()
    no_formatter: list[Path] = []
    for f in files:
        resolved: Path
        if not f.exists():
            logger().debug(f"File {f!r} does not exist (deleted?), skipping.")
            continue
        if f.is_symlink():
            logger().debug(f"File {f!r} is a symlink, skipping.")
            continue
        try:
            resolved = f.resolve(strict=True)
        except OSError:
            logger().warn(f"File {f!r} cannot be resolved, skipping.")
            continue
        formatter = find_formatter(resolved)
        if formatter is None:
            no_formatter.append(resolved)
            continue
        if resolved.suffix != f.suffix:
            logger().warn(
                f"File {f!r} has extension {f.suffix!r}, but resolves "
                f"to {resolved!r} with extension {resolved.suffix!r}. "
                f"Selected formatter: {formatter.name}."
            )
        result.add(resolved)
    return result, no_formatter


@dataclass(frozen=True)
class FormatResult:
    """Result of formatting a chunk of files."""

    formatter_name: str
    modified_files: list[Path]
    # Absolute perf_counter timestamps for this chunk, used to compute
    # wall-clock time per formatter across parallel chunks.
    start_time: float
    end_time: float
    error: FormatterError | None


def format_files(
    formatter: Formatter,
    chunk: Sequence[Path],
    log: Logger,
) -> FormatResult:
    """
    Format a chunk of files and return the result.

    The result includes modified files count, the time taken, and the
    formatter name.
    """
    set_logger(log)
    start = time.perf_counter()
    mtimes = {f: os.path.getmtime(f) for f in chunk}
    error: FormatterError | None = None
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


def main() -> None:
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

    if not args.all_tracked and not args.file and not args.git and not args.jj:
        try:
            jj_repo_root()
            args.jj = True
        except (subprocess.CalledProcessError, FileNotFoundError, ValueError):
            try:
                git_repo_root()
                args.git = True
            except (
                subprocess.CalledProcessError,
                FileNotFoundError,
                ValueError,
            ):
                pass

    result = collect_files(cast(_CollectArgs, args))
    if result is None:
        print("No input sources specified.", file=sys.stderr)
        p.print_usage(file=sys.stderr)
        sys.exit(1)

    files, unsupported_ext_files = result
    files, no_shebang_files = resolve_files(files)
    files = {f for f in files if not _registry().ignore.matches(f)}

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
                f"No formatter found for file {f!r} with extension {f.suffix!r}"
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

    no_formatter_files = [
        f
        for f in unsupported_ext_files + no_shebang_files
        if not _registry().ignore.matches(f)
    ]
    if no_formatter_files:
        exts = sorted(
            {f.suffix or "<no-extension>" for f in no_formatter_files}
        )
        logger().warn(
            f"{len(no_formatter_files)} file(s) had no associated formatter "
            f"(extensions: {', '.join(exts)})"
        )
        for f in sorted(no_formatter_files):
            shebang = _read_shebang(f)
            suffix = f"  [{shebang}]" if shebang else ""
            logger().warn(f"  {f.relative_to(cwd)}{suffix}")

    if logger().is_debug_enabled:
        for name, (start, end) in formatter_spans.items():
            logger().debug(f"Time spent in {name}: {end - start:.2f}s")


if __name__ == "__main__":
    main()
