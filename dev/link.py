#!/usr/bin/env python3
"""
Symlinks files from config/ to $HOME, mirroring the directory structure.

Usage: python dev/link.py [--dry-run] [--verbose] [--unlink] [--force]
"""

import argparse
import base64
import filecmp
import os
import shutil
import subprocess
import sys
from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import TextIO

try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ImportError:
        tomllib = None  # type: ignore[assignment]

from util import (
    BOLD,
    GRAY,
    GREEN,
    MAGENTA,
    RED,
    RESET,
    YELLOW,
    PROJECT_ROOT,
    require_project_root,
)


CONFIG_DIR = "home"
LINK_TOML = ".link.toml"
LINK_STATE = ".link-state.toml"
ARROW = "⟶"
INDENT = "    "

FILE_MODES = ("symlink", "copy", "windows-symlink")


@dataclass(frozen=True)
class FileConfig:
    dest: Path | None = None
    skip: bool = False
    mode: str = "symlink"  # "symlink", "copy", or "windows-symlink"


@dataclass(frozen=True)
class _StatusMetadata:
    color: str
    file: TextIO
    verbose: bool
    symbol: str


class Logger:
    """Formats and prints status lines for link operations."""

    class Status(Enum):
        LINK = _StatusMetadata(GREEN, sys.stdout, verbose=False, symbol="+")
        UNLINK = _StatusMetadata(YELLOW, sys.stdout, verbose=False, symbol="-")
        ERROR = _StatusMetadata(RED, sys.stderr, verbose=False, symbol="!")
        EXISTS = _StatusMetadata(GRAY, sys.stdout, verbose=True, symbol="=")
        ABSENT = _StatusMetadata(GRAY, sys.stdout, verbose=True, symbol=" ")

    def __init__(self, verbose: bool):
        self.verbose = verbose
        self.target_count = 0

    def link(self, dest: Path, src: Path) -> None:
        self.target_count += 1
        self._log(
            self.Status.LINK,
            f"{GREEN}{self.fmt_path(dest)}{RESET} {MAGENTA}{ARROW} {self.fmt_path(src)}{RESET}",
        )

    def unlinked(self, dest: Path) -> None:
        self._log(self.Status.UNLINK, f"{YELLOW}{self.fmt_path(dest)}{RESET}")

    def exists(self, dest: Path) -> None:
        self.target_count += 1
        self._log(self.Status.EXISTS, f"{GRAY}{self.fmt_path(dest)}{RESET}")

    def absent(self, dest: Path) -> None:
        self._log(self.Status.ABSENT, f"{GRAY}{self.fmt_path(dest)}{RESET}")

    def removed_orphan(self, dest: Path, src: Path) -> None:
        self._log(
            self.Status.UNLINK,
            f"{YELLOW}{self.fmt_path(dest)}{RESET} {GRAY}(removed because {self.fmt_path(src)} no longer exists){RESET}",
        )

    def error(self, message: str) -> None:
        self._log(self.Status.ERROR, message)

    def forced(self, dest: Path, src: Path) -> None:
        self._log(
            self.Status.LINK,
            f"{GREEN}{self.fmt_path(dest)}{RESET} {MAGENTA}{ARROW} {self.fmt_path(src)}{RESET} {RED}(--force){RESET}",
        )

    def notice(self, message: str) -> None:
        print(f"{GRAY}{message}{RESET}", file=sys.stderr)

    def fmt_path(self, path: Path) -> str:
        """Format path for display, substituting ~ for $HOME."""
        home = Path.home()
        try:
            return "~/" + str(path.relative_to(home))
        except ValueError:
            return str(path)

    def _log(self, status: Status, message: str) -> None:
        data = status.value
        if data.verbose and not self.verbose:
            return
        print(
            f"{BOLD}{data.color}[{data.symbol}]{RESET} {message}",
            file=data.file,
        )


def _toml_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


class LinkState:
    """
    Persists the set of symlinks managed by this script.

    Stored as .link-state.toml in the repo root. Entries are added when a
    symlink is created or verified, and removed when it is deleted. This lets
    --unlink clean up symlinks whose source files were later deleted from the
    repo.
    """

    def __init__(self, path: Path):
        self.path = path
        self._links: dict[str, str] = {}  # str(dest_abs) -> str(src_abs)

    def load(self) -> None:
        if not self.path.exists():
            return
        if tomllib is None:
            return
        with open(self.path, "rb") as f:
            data = tomllib.load(f)
        self._links = dict(data.get("links", {}))

    def save(self) -> None:
        lines = ["[links]\n"]
        for dest in sorted(self._links):
            src = self._links[dest]
            lines.append(f'"{_toml_escape(dest)}" = "{_toml_escape(src)}"\n')
        self.path.write_text("".join(lines))

    def add(self, dest: Path, src: Path) -> None:
        self._links[str(dest)] = str(src)

    def remove(self, dest: Path) -> None:
        self._links.pop(str(dest), None)

    def items(self) -> list[tuple[Path, Path]]:
        return [(Path(d), Path(s)) for d, s in self._links.items()]


class Linker:
    """
    Performs (or simulates) symlink operations.

    All operations respect dry_run and force, and track whether any unresolved
    conflicts occurred.
    """

    def __init__(
        self,
        dry_run: bool,
        force: bool,
        logger: Logger,
        state: LinkState,
        repo_root: Path | None = None,
    ):
        self.dry_run = dry_run
        self.force = force
        self.logger = logger
        self.state = state
        self.repo_root = repo_root
        self.had_conflict = False

    def link_file(self, src: Path, dest: Path) -> None:
        """Symlink dest -> src (src is a file)."""
        self._link(src, dest)

    def link_dir(self, src: Path, dest: Path) -> None:
        """Symlink dest -> src (src is a directory)."""
        self._link(src, dest)

    def unlink_file(self, src: Path, dest: Path) -> None:
        """Remove symlink at dest if it points to src."""
        self._unlink(src, dest)

    def unlink_dir(self, src: Path, dest: Path) -> None:
        """Remove symlink at dest if it points to src."""
        self._unlink(src, dest)

    def _link(self, src: Path, dest: Path) -> None:
        src_abs = src.resolve()

        if dest.is_symlink():
            actual = dest.resolve()
            if actual == src_abs:
                if not self.dry_run:
                    self.state.add(dest, actual)
                self.logger.exists(dest)
                return
            self.logger.error(
                f"{RED}{self.logger.fmt_path(dest)}{RESET}: symlink exists but points to wrong file.\n"
                f"{GRAY}{INDENT}current:  {self.logger.fmt_path(actual)}\n"
                f"{GRAY}{INDENT}expected: {self.logger.fmt_path(src_abs)}{RESET}"
                + (
                    f"\n{GRAY}{INDENT}--force provided, overriding to {MAGENTA}{ARROW} {self.logger.fmt_path(src_abs)}{RESET}"
                    if self.force
                    else f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                )
            )
            if not self.force:
                self.had_conflict = True
                return
            if not self.dry_run:
                dest.unlink()
                dest.symlink_to(src_abs)
                self.state.add(dest, src_abs)
            return

        if dest.exists():
            _hint = (
                f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                if dest.is_dir()
                else (
                    f"\n{GRAY}{INDENT}--force provided, overriding to {MAGENTA}{ARROW} {self.logger.fmt_path(src_abs)}{RESET}"
                    if self.force
                    else f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                )
            )
            self.logger.error(
                f"{RED}{self.logger.fmt_path(dest)}{RESET}: exists and is not a symlink{_hint}"
            )
            if not self.force or dest.is_dir():
                self.had_conflict = True
                return
            if not self.dry_run:
                dest.unlink()
                dest.parent.mkdir(parents=True, exist_ok=True)
                dest.symlink_to(src_abs)
                self.state.add(dest, src_abs)
            return

        # dest is absent — create symlink
        if not self.dry_run:
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.symlink_to(src_abs)
            self.state.add(dest, src_abs)
        self.logger.link(dest, src_abs)

    def _unlink(self, src: Path, dest: Path) -> None:
        if dest.is_symlink():
            actual = dest.resolve()
            expected = src.resolve()
            if actual != expected:
                self.logger.error(
                    f"{RED}{self.logger.fmt_path(dest)}{RESET}: symlink points to wrong file — skipping.\n"
                    f"{GRAY}{INDENT}current:  {self.logger.fmt_path(actual)}\n"
                    f"{GRAY}{INDENT}expected: {self.logger.fmt_path(expected)}{RESET}"
                    + (
                        ""
                        if self.force
                        else f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                    )
                )
                if not self.force:
                    self.had_conflict = True
                    return
            if not self.dry_run:
                dest.unlink()
                self.state.remove(dest)
            self.logger.unlinked(dest)
        elif dest.exists():
            self._conflict(
                f"{RED}{self.logger.fmt_path(dest)}{RESET}: exists and is not a symlink"
                " — cannot unlink"
            )
        else:
            self.logger.absent(dest)

    def _conflict(self, message: str) -> None:
        self.logger.error(
            message + f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
        )
        self.had_conflict = True


class LinkMode(ABC):
    """
    Interface for link modes. Each mode controls how a directory is deployed.

    Modes are registered in MODES and selected via .link.toml marker files.
    Custom modes can be added to MODES by users.
    """

    @abstractmethod
    def link(self, src: Path, dest: Path, linker: Linker) -> None:
        """Deploy src directory to dest using linker for all operations."""
        ...

    @abstractmethod
    def unlink(self, src: Path, dest: Path, linker: Linker) -> None:
        """Undo the deployment created by link()."""
        ...


class SymlinkFilesMode(LinkMode):
    """
    Default mode. Recurses into src and creates a symlink for each file.
    Subdirectories are created (not symlinked) at dest; sub-subdirectories
    with their own .link.toml use that mode instead.
    After unlinking, empty destination directories are removed.
    """

    def link(self, src: Path, dest: Path, linker: Linker) -> None:
        _walk(src, dest, linker, unlink=False)

    def unlink(self, src: Path, dest: Path, linker: Linker) -> None:
        _walk(src, dest, linker, unlink=True)
        if not linker.dry_run and dest.is_dir() and not dest.is_symlink():
            try:
                dest.rmdir()  # only succeeds if now empty
            except OSError:
                pass


class SymlinkDirMode(LinkMode):
    """
    Links the entire directory as a single symlink (no recursion).

    Use a .link.toml with mode = "symlink-dir" in a directory to activate.
    """

    def link(self, src: Path, dest: Path, linker: Linker) -> None:
        linker.link_dir(src, dest)

    def unlink(self, src: Path, dest: Path, linker: Linker) -> None:
        linker.unlink_dir(src, dest)


class IgnoreMode(LinkMode):
    """Skips the directory entirely — neither links nor unlinks anything."""

    def link(self, src: Path, dest: Path, linker: "Linker") -> None:
        pass

    def unlink(self, src: Path, dest: Path, linker: "Linker") -> None:
        pass


class CopyFilesMode(LinkMode):
    """
    Copies each file from src into dest (no symlinking).

    Use for files that Windows applications require as real files rather than
    symlinks (e.g. .lnk shortcuts in the Windows Startup folder).
    Activate with mode = "copy-files" in .link.toml.
    """

    def link(self, src: Path, dest: Path, linker: "Linker") -> None:
        self._walk(src, dest, linker, unlink=False)

    def unlink(self, src: Path, dest: Path, linker: "Linker") -> None:
        self._walk(src, dest, linker, unlink=True)

    def _walk(
        self, src: Path, dest: Path, linker: "Linker", unlink: bool
    ) -> None:
        entries = [e for e in sorted(src.iterdir()) if e.name != LINK_TOML]
        if linker.repo_root is not None:
            ignored = _git_ignored(entries, linker.repo_root)
            entries = [e for e in entries if e not in ignored]
        for entry in entries:
            dest_entry = dest / entry.name
            if entry.is_dir() and not entry.is_symlink():
                self._walk(entry, dest_entry, linker, unlink)
            elif entry.is_file() or entry.is_symlink():
                if unlink:
                    self._uncopy(entry, dest_entry, linker)
                else:
                    self._copy(entry, dest_entry, linker)

    def _copy(self, src: Path, dest: Path, linker: "Linker") -> None:
        if dest.exists():
            if filecmp.cmp(src, dest, shallow=False):
                linker.logger.exists(dest)
                return
            linker._conflict(
                f"{RED}{linker.logger.fmt_path(dest)}{RESET}: exists with different content"
            )
            return
        if not linker.dry_run:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dest)
            linker.state.add(dest, src.resolve())
        linker.logger.link(dest, src.resolve())

    def _uncopy(self, src: Path, dest: Path, linker: "Linker") -> None:
        if dest.exists():
            if not linker.dry_run:
                dest.unlink()
                linker.state.remove(dest)
            linker.logger.unlinked(dest)
        else:
            linker.logger.absent(dest)


def _win_path_from_mnt(path: Path) -> str | None:
    """Convert /mnt/X/... to X:\\... Windows path, or None if not a /mnt/ path."""
    parts = path.parts  # ('/', 'mnt', 'c', ...)
    if len(parts) >= 3 and parts[1] == "mnt" and len(parts[2]) == 1:
        drive = parts[2].upper()
        rest = "\\".join(parts[3:])
        return f"{drive}:\\{rest}" if rest else f"{drive}:\\"
    return None


def _wsl_to_unc(path: Path) -> str:
    """Convert an absolute WSL path to a \\\\wsl.localhost\\<distro>\\... UNC path."""
    distro = os.environ.get("WSL_DISTRO_NAME", "Ubuntu")
    return (
        "\\\\wsl.localhost\\" + distro + str(path.resolve()).replace("/", "\\")
    )


def _run_ps(cmd: str) -> "subprocess.CompletedProcess[bytes]":
    enc = base64.b64encode(cmd.encode("utf-16-le")).decode("ascii")
    return subprocess.run(
        [
            "powershell.exe",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            enc,
        ],
        capture_output=True,
    )


def _run_ps_elevated(cmd: str) -> "subprocess.CompletedProcess[bytes]":
    enc = base64.b64encode(cmd.encode("utf-16-le")).decode("ascii")
    outer = (
        "Start-Process powershell -Verb RunAs -Wait "
        f"-ArgumentList '-NoProfile -ExecutionPolicy Bypass -EncodedCommand {enc}'"
    )
    return _run_ps(outer)


class WindowsSymlinkDirMode(LinkMode):
    """
    Creates a Windows junction/symlink from a Windows path to the WSL source directory.

    Required when dest is on the Windows filesystem (e.g. /mnt/c/...), because
    regular WSL symlinks are not recognized by Windows applications.  Uses
    elevated PowerShell (New-Item -ItemType SymbolicLink).

    Activate with mode = "windows-symlink-dir" in .link.toml.
    """

    def link(self, src: Path, dest: Path, linker: "Linker") -> None:
        win_dest = _win_path_from_mnt(dest)
        if win_dest is None:
            linker._conflict(
                f"{RED}{linker.logger.fmt_path(dest)}{RESET}: windows-symlink-dir requires "
                "a /mnt/<drive>/... destination"
            )
            return
        unc_src = _wsl_to_unc(src)

        ps_result = _run_ps(
            f'$i = Get-Item "{win_dest}" -ErrorAction SilentlyContinue\n'
            f"if (-not $i) {{ exit 2 }}\n"
            f'if ($i.LinkType -ne "SymbolicLink") {{ exit 3 }}\n'
            f'$actual = ($i.Target -join "").TrimEnd("\\")\n'
            f'if ($actual.StartsWith("UNC\\")) {{ $actual = "\\\\" + $actual.Substring(4) }}\n'
            f'$expected = "{unc_src}".TrimEnd("\\")\n'
            f"if ($actual -eq $expected) {{ exit 0 }}\n"
            f"Write-Output $actual\n"
            f"exit 1"
        )
        rc = ps_result.returncode

        if rc == 0:
            if not linker.dry_run:
                linker.state.add(dest, src.resolve())
            linker.logger.exists(dest)
            return
        if rc == 3:
            linker.logger.error(
                f"{RED}{linker.logger.fmt_path(dest)}{RESET}: exists on Windows and is not a symlink"
                + (
                    f"\n{GRAY}{INDENT}--force provided, overriding to {MAGENTA}{ARROW} {linker.logger.fmt_path(src.resolve())}{RESET}"
                    if linker.force
                    else f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                )
            )
            if not linker.force:
                linker.had_conflict = True
                return
            if not linker.dry_run:
                linker.logger.notice(
                    f"(requesting Windows administrator access to remove {win_dest} — a UAC prompt will appear)"
                )
                result = _run_ps_elevated(
                    f'Remove-Item "{win_dest}" -Force -Recurse'
                )
                if result.returncode != 0:
                    linker._conflict(
                        f"{RED}{linker.logger.fmt_path(dest)}{RESET}: failed to remove existing Windows item"
                    )
                    return
        elif rc == 1:
            actual_win = ps_result.stdout.decode(errors="replace").strip()
            linker.logger.error(
                f"{RED}{linker.logger.fmt_path(dest)}{RESET}: Windows symlink exists but points to wrong file.\n"
                f"{GRAY}{INDENT}current:  {actual_win}\n"
                f"{GRAY}{INDENT}expected: {unc_src}{RESET}"
                + (
                    f"\n{GRAY}{INDENT}--force provided, overriding to {MAGENTA}{ARROW} {linker.logger.fmt_path(src.resolve())}{RESET}"
                    if linker.force
                    else f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                )
            )
            if not linker.force:
                linker.had_conflict = True
                return
            if not linker.dry_run:
                linker.logger.notice(
                    f"(requesting Windows administrator access to remove {win_dest} — a UAC prompt will appear)"
                )
                result = _run_ps_elevated(f'Remove-Item "{win_dest}" -Force')
                if result.returncode != 0:
                    linker._conflict(
                        f"{RED}{linker.logger.fmt_path(dest)}{RESET}: failed to remove existing Windows symlink"
                    )
                    return
        # rc == 2: absent; rc == 1 or 3 with --force: create it
        if not linker.dry_run:
            linker.logger.notice(
                f"(requesting Windows administrator access to create symlink at {win_dest} — a UAC prompt will appear)"
            )
            result = _run_ps_elevated(
                f'New-Item -ItemType SymbolicLink -Path "{win_dest}" -Target "{unc_src}" -Force'
            )
            if result.returncode != 0:
                linker._conflict(
                    f"{RED}{linker.logger.fmt_path(dest)}{RESET}: failed to create Windows symlink"
                )
                return
            linker.state.add(dest, src.resolve())
        if rc == 2:
            linker.logger.link(dest, src.resolve())
        else:
            linker.logger.forced(dest, src.resolve())

    def unlink(self, src: Path, dest: Path, linker: "Linker") -> None:
        win_dest = _win_path_from_mnt(dest)
        if win_dest is None:
            linker._conflict(
                f"{RED}{linker.logger.fmt_path(dest)}{RESET}: windows-symlink-dir requires "
                "a /mnt/<drive>/... destination"
            )
            return

        if (
            _run_ps(
                f'if (Test-Path "{win_dest}") {{ exit 0 }} else {{ exit 1 }}'
            ).returncode
            != 0
        ):
            linker.logger.absent(dest)
            return

        if not linker.dry_run:
            linker.logger.notice(
                f"(requesting Windows administrator access to remove {win_dest} — a UAC prompt will appear)"
            )
            result = _run_ps_elevated(f'Remove-Item "{win_dest}" -Force')
            if result.returncode != 0:
                linker._conflict(
                    f"{RED}{linker.logger.fmt_path(dest)}{RESET}: failed to remove Windows symlink"
                )
                return
            linker.state.remove(dest)
        linker.logger.unlinked(dest)


MODES: dict[str, LinkMode] = {
    "symlink-files": SymlinkFilesMode(),
    "symlink-dir": SymlinkDirMode(),
    "windows-symlink-dir": WindowsSymlinkDirMode(),
    "copy-files": CopyFilesMode(),
    "ignore": IgnoreMode(),
}

DEFAULT_MODE = "symlink-files"


def _parse_link_target(
    spec: dict, link_toml: Path, logger: Logger
) -> tuple[LinkMode, Path | None]:
    """Parse a single link target spec dict into (mode, dest_override)."""
    mode_name: str = spec.get("mode", DEFAULT_MODE)
    if mode_name not in MODES:
        logger.error(
            f"{MAGENTA}{logger.fmt_path(link_toml)}{RESET}: unknown mode {repr(mode_name)}. "
            f"Available modes: {', '.join(MODES)}"
        )
        sys.exit(1)
    dest_str: str | None = spec.get("dest")
    dest_override = Path(dest_str).expanduser() if dest_str else None
    return MODES[mode_name], dest_override


def _read_mode(
    directory: Path, logger: Logger
) -> tuple[list[tuple[LinkMode, Path | None]], dict[str, FileConfig]]:
    """Read .link.toml from directory and return a list of (mode, dest_override)
    targets and per-file configs.

    Supports multiple targets via [[link.targets]] in addition to the primary
    [link] section.
    """
    link_toml = directory / LINK_TOML
    if not link_toml.exists():
        return [(MODES[DEFAULT_MODE], None)], {}

    if tomllib is None:
        logger.error(
            f"cannot read {MAGENTA}{logger.fmt_path(link_toml)}{RESET}: "
            "tomllib unavailable (Python 3.11+ required, or install tomli)"
        )
        sys.exit(1)

    with open(link_toml, "rb") as f:
        data = tomllib.load(f)

    link_section = data.get("link", {})
    targets: list[tuple[LinkMode, Path | None]] = [
        _parse_link_target(link_section, link_toml, logger)
    ]
    for extra in link_section.get("targets", []):
        targets.append(_parse_link_target(extra, link_toml, logger))

    file_configs: dict[str, FileConfig] = {}
    for name, cfg in data.get("files", {}).items():
        if not isinstance(cfg, dict):
            continue
        file_dest_str: str | None = cfg.get("dest")
        file_dest = Path(file_dest_str).expanduser() if file_dest_str else None
        skip = bool(cfg.get("skip", False))
        file_mode: str = cfg.get("mode", "symlink")
        if file_mode not in FILE_MODES:
            logger.error(
                f"{MAGENTA}{logger.fmt_path(link_toml)}{RESET}: unknown file mode {repr(file_mode)}"
                f" for {repr(name)}. Available: {', '.join(FILE_MODES)}"
            )
            sys.exit(1)
        file_configs[name] = FileConfig(
            dest=file_dest, skip=skip, mode=file_mode
        )

    return targets, file_configs


def _git_ignored(paths: list[Path], repo_root: Path) -> set[Path]:
    """Return the subset of paths that git considers ignored."""
    if not paths:
        return set()
    result = subprocess.run(
        ["git", "check-ignore", "--"] + [str(p) for p in paths],
        capture_output=True,
        text=True,
        cwd=repo_root,
    )
    return {Path(line) for line in result.stdout.splitlines() if line}


def _copy_file(src: Path, dest: Path, linker: Linker) -> None:
    if dest.exists():
        if filecmp.cmp(src, dest, shallow=False):
            linker.logger.exists(dest)
            return
        linker._conflict(
            f"{RED}{linker.logger.fmt_path(dest)}{RESET}: exists with different content"
        )
        return
    if not linker.dry_run:
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, dest)
        linker.state.add(dest, src.resolve())
    linker.logger.link(dest, src.resolve())


def _uncopy_file(dest: Path, linker: Linker) -> None:
    if dest.exists():
        if not linker.dry_run:
            dest.unlink()
            linker.state.remove(dest)
        linker.logger.unlinked(dest)
    else:
        linker.logger.absent(dest)


def _walk(src: Path, dest: Path, linker: Linker, unlink: bool) -> None:
    """
    Walk src directory. For each entry:
      - .link.toml files: skip (they're config, not content).
      - Regular files / file-symlinks: link or unlink directly.
      - Directories: read their .link.toml, dispatch to appropriate mode.
    Per-file overrides are read from src/.link.toml [files.*] sections.
    """
    _, file_configs = _read_mode(src, linker.logger)
    entries = [e for e in sorted(src.iterdir()) if e.name != LINK_TOML]
    if linker.repo_root is not None:
        ignored = _git_ignored(entries, linker.repo_root)
        entries = [e for e in entries if e not in ignored]
    for entry in entries:
        dest_entry = dest / entry.name
        if entry.is_dir() and not entry.is_symlink():
            targets, _ = _read_mode(entry, linker.logger)
            for mode, dest_override in targets:
                effective_dest = (
                    dest_override if dest_override is not None else dest_entry
                )
                if unlink:
                    mode.unlink(entry, effective_dest, linker)
                else:
                    mode.link(entry, effective_dest, linker)
        elif entry.is_file() or entry.is_symlink():
            file_cfg = file_configs.get(entry.name)
            if file_cfg and file_cfg.skip:
                continue
            is_win = file_cfg is not None and file_cfg.mode == "windows-symlink"
            effective_dest = (
                dest_entry
                if is_win
                else (
                    file_cfg.dest
                    if (file_cfg and file_cfg.dest)
                    else dest_entry
                )
            )
            if unlink:
                if is_win:
                    MODES["windows-symlink-dir"].unlink(
                        entry,
                        file_cfg.dest,
                        linker,  # type: ignore[union-attr]
                    )
                    linker.unlink_file(entry, effective_dest)
                elif file_cfg and file_cfg.mode == "copy":
                    _uncopy_file(effective_dest, linker)
                else:
                    linker.unlink_file(entry, effective_dest)
            else:
                if is_win:
                    linker.link_file(entry, effective_dest)
                    MODES["windows-symlink-dir"].link(
                        entry,
                        file_cfg.dest,
                        linker,  # type: ignore[union-attr]
                    )
                elif file_cfg and file_cfg.mode == "copy":
                    _copy_file(entry, effective_dest, linker)
                else:
                    linker.link_file(entry, effective_dest)


def main() -> None:
    require_project_root()

    config_dir = PROJECT_ROOT / CONFIG_DIR

    p = argparse.ArgumentParser(
        description=f"Symlink {CONFIG_DIR}/ files into $HOME.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would be done without making any changes.",
    )
    p.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Also log already-correct links.",
    )
    p.add_argument(
        "--unlink",
        action="store_true",
        help="Remove managed symlinks instead of creating them.",
    )
    p.add_argument(
        "--force",
        action="store_true",
        help="Overwrite conflicting files/symlinks instead of aborting.",
    )
    args = p.parse_args()

    logger = Logger(verbose=args.verbose)

    state = LinkState(PROJECT_ROOT / LINK_STATE)
    state.load()

    linker = Linker(
        dry_run=args.dry_run,
        force=args.force,
        logger=logger,
        state=state,
        repo_root=PROJECT_ROOT,
    )

    if args.dry_run:
        logger.notice("(--dry-run: no changes will be made)")
    _walk(config_dir, Path.home(), linker, unlink=args.unlink)
    if args.unlink:
        # Unlink stale entries: src files removed from repo since last link run.
        for dest, src in state.items():
            linker.unlink_file(src, dest)
    else:
        # Remove orphaned symlinks: src deleted from repo, dest still exists.
        for dest, src in state.items():
            if not src.exists():
                if not args.dry_run and dest.is_symlink():
                    dest.unlink()
                    state.remove(dest)
                logger.removed_orphan(dest, src)
    if args.dry_run:
        logger.notice("(--dry-run: no changes have been made)")

    if not args.unlink:
        print(f"{GREEN}✓ {logger.target_count} targets linked{RESET}")

    if not args.dry_run:
        state.save()

    if linker.had_conflict:
        if args.force:
            logger.error(
                f"{RED}some conflicts must be resolved manually{RESET}"
            )
        else:
            logger.error(f"{RED}conflicts found{RESET}")
        sys.exit(1)


if __name__ == "__main__":
    main()
