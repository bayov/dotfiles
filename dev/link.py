#!/usr/bin/env python3
"""
Symlinks files from home/ to $HOME, mirroring the directory structure.

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
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Callable, ClassVar, TextIO

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
TAGS_TOML = "link.tags.toml"
LINKS_STATE = ".links.toml"
ARROW = "⟶"
INDENT = "    "


class LinkerType(str, Enum):
    SYMLINK = "symlink"
    SPREAD = "spread"
    RECURSIVE = "recursive"
    COPY = "copy"
    WINDOWS_SYMLINK = "windows-symlink"
    IGNORE = "ignore"


@dataclass(frozen=True)
class Link:
    src: Path  # absolute, resolved
    dst: Path | None = None  # None = infer from parent context
    linker: LinkerType = LinkerType.SYMLINK
    tags: tuple[
        str, ...
    ] = ()  # empty = always applies; AND semantics; "!tag" = negation


@dataclass(frozen=True)
class DirConfig:
    links: list[
        Link
    ]  # all [[link]] entries; no-src entries have src = directory.resolve()


# ---------------------------------------------------------------------------
# Linker result types
# ---------------------------------------------------------------------------


@dataclass
class LinkerError:
    """The link operation failed (conflict or unresolvable error)."""

    message: str


@dataclass
class LinkerSuccess:
    """The link operation completed (or was already in the correct state)."""

    changed: bool = (
        True  # False = already correct (link) / already absent (unlink)
    )
    forced: bool = False  # True = overrode a conflicting file via --force


@dataclass
class LinkerDelegate:
    """Delegate to sub-links; the orchestrator dispatches each in turn."""

    links: list[Link]
    post_action: Callable[[], None] | None = (
        None  # called after all links are dispatched
    )


LinkerResult = LinkerError | LinkerSuccess | LinkerDelegate


# ---------------------------------------------------------------------------
# Logger
# ---------------------------------------------------------------------------


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

    def link(self, dst: Path, src: Path) -> None:
        self.target_count += 1
        self._log(
            self.Status.LINK,
            f"{GREEN}{self.fmt_path(dst)}{RESET} {MAGENTA}{ARROW} {self.fmt_path(src)}{RESET}",
        )

    def unlinked(self, dst: Path) -> None:
        self._log(self.Status.UNLINK, f"{YELLOW}{self.fmt_path(dst)}{RESET}")

    def exists(self, dst: Path) -> None:
        self.target_count += 1
        self._log(self.Status.EXISTS, f"{GRAY}{self.fmt_path(dst)}{RESET}")

    def absent(self, dst: Path) -> None:
        self._log(self.Status.ABSENT, f"{GRAY}{self.fmt_path(dst)}{RESET}")

    def removed_orphan(self, dst: Path, src: Path) -> None:
        self._log(
            self.Status.UNLINK,
            f"{YELLOW}{self.fmt_path(dst)}{RESET} "
            f"{GRAY}(removed because {self.fmt_path(src)} no longer exists){RESET}",
        )

    def error(self, message: str) -> None:
        self._log(self.Status.ERROR, message)

    def forced(self, dst: Path, src: Path) -> None:
        self._log(
            self.Status.LINK,
            f"{GREEN}{self.fmt_path(dst)}{RESET} {MAGENTA}{ARROW} {self.fmt_path(src)}{RESET} "
            f"{RED}(--force){RESET}",
        )

    def notice(self, message: str) -> None:
        print(f"{YELLOW}{message}{RESET}", file=sys.stderr)

    def fmt_path(self, path: Path) -> str:
        home = Path.home()
        try:
            return "~/" + str(path.relative_to(home))
        except ValueError:
            return str(path)

    def _log(self, status: "Logger.Status", message: str) -> None:
        data = status.value
        if data.verbose and not self.verbose:
            return
        print(
            f"{BOLD}{data.color}[{data.symbol}]{RESET} {message}",
            file=data.file,
        )


# ---------------------------------------------------------------------------
# State
# ---------------------------------------------------------------------------


def _toml_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


class LinksState:
    """
    Persists the set of links managed by this script.

    Stored as .links.toml in the repo root. Entries are added when a link is
    created or verified, and removed when deleted. This lets --unlink clean up
    links whose source files were later deleted from the repo.
    """

    def __init__(self, path: Path):
        self.path = path
        self._links: dict[str, dict] = {}  # str(dst) -> {src, linker}

    def load(self) -> None:
        if not self.path.exists():
            return
        if tomllib is None:
            return
        with open(self.path, "rb") as f:
            data = tomllib.load(f)
        for entry in data.get("links", []):
            dst = entry.get("dst", "")
            src = entry.get("src", "")
            linker = entry.get("linker", LinkerType.SYMLINK.value)
            if dst:
                self._links[dst] = {"src": src, "linker": linker}

    def save(self) -> None:
        parts = []
        for dst in sorted(self._links):
            entry = self._links[dst]
            parts.append(
                f"[[links]]\n"
                f'dst = "{_toml_escape(dst)}"\n'
                f'src = "{_toml_escape(entry["src"])}"\n'
                f'linker = "{_toml_escape(entry["linker"])}"\n'
            )
        self.path.write_text("\n".join(parts))

    def add(self, link: "Link") -> None:
        assert link.dst is not None
        self._links[str(link.dst)] = {
            "src": str(link.src),
            "linker": link.linker.value,
        }

    def remove(self, dst: Path) -> None:
        self._links.pop(str(dst), None)

    def items(self) -> list[Link]:
        result = []
        for dst_str, entry in self._links.items():
            try:
                linker = LinkerType(entry["linker"])
            except ValueError:
                linker = LinkerType.SYMLINK
            result.append(
                Link(
                    src=Path(entry["src"]),
                    dst=Path(dst_str),
                    linker=linker,
                )
            )
        return result


# ---------------------------------------------------------------------------
# Context
# ---------------------------------------------------------------------------


@dataclass
class Context:
    dry_run: bool
    force: bool
    logger: Logger
    state: LinksState
    repo_root: Path | None
    had_conflict: bool = False
    active_tags: frozenset[str] = field(default_factory=frozenset)
    _visited_link_tomls: set[Path] = field(default_factory=set)


# ---------------------------------------------------------------------------
# Linker ABC
# ---------------------------------------------------------------------------


class Linker(ABC):
    name: ClassVar[LinkerType]

    @abstractmethod
    def link(self, link: Link, ctx: Context) -> LinkerResult:
        """Deploy link.src to link.dst, or return sub-links to dispatch."""
        ...

    @abstractmethod
    def unlink(self, link: Link, ctx: Context) -> LinkerResult:
        """Undo the deployment created by link()."""
        ...


# ---------------------------------------------------------------------------
# Tag detection
# ---------------------------------------------------------------------------


def load_active_tags(tags_toml: Path) -> frozenset[str]:
    """Read link.tags.toml and return the set of tags whose detect commands exit 0."""
    if not tags_toml.exists() or tomllib is None:
        return frozenset()
    with open(tags_toml, "rb") as f:
        data = tomllib.load(f)
    active: set[str] = set()
    for tag_name, tag_cfg in data.get("tags", {}).items():
        detect_cmd: str | None = tag_cfg.get("detect")
        if detect_cmd is None:
            continue
        r = subprocess.run(
            detect_cmd, shell=True, executable="/bin/sh", capture_output=True
        )
        if r.returncode == 0:
            active.add(tag_name)
    return frozenset(active)


def _link_matches_tags(link: Link, active_tags: frozenset[str]) -> bool:
    """Return True if link's tags filter matches active_tags.

    Empty tags = always matches. All listed tags must be active (AND).
    Tags prefixed '!' are negated (that tag must NOT be active).
    """
    for tag in link.tags:
        if tag.startswith("!"):
            if tag[1:] in active_tags:
                return False
        else:
            if tag not in active_tags:
                return False
    return True


def _filter_by_tags(
    entries: list[Link], active_tags: frozenset[str]
) -> list[Link]:
    """Return the subset of entries that match active_tags.

    If any entry has tags and none match → return [] (explicit env control).
    If all entries are tagless → return all (backward compat).
    """
    matching = [e for e in entries if _link_matches_tags(e, active_tags)]
    return matching if (matching or any(e.tags for e in entries)) else entries


# ---------------------------------------------------------------------------
# Config parsing
# ---------------------------------------------------------------------------


def _read_dir_config(directory: Path, ctx: Context) -> DirConfig:
    """
    Read .link.toml from directory and return DirConfig.
    Marks the file as visited in ctx._visited_link_tomls.
    """
    link_toml = directory / LINK_TOML
    if not link_toml.exists():
        return DirConfig(links=[])

    ctx._visited_link_tomls.add(link_toml)

    if tomllib is None:
        ctx.logger.error(
            f"cannot read {MAGENTA}{ctx.logger.fmt_path(link_toml)}{RESET}: "
            "tomllib unavailable (Python 3.11+ required, or install tomli)"
        )
        sys.exit(1)

    with open(link_toml, "rb") as f:
        data = tomllib.load(f)

    links: list[Link] = []
    for entry in data.get("link", []):
        src_str: str | None = entry.get("src")

        if src_str is None:
            # No-src entry: refers to the directory itself (the primary link).
            resolved_src = directory.resolve()
        else:
            src_path = Path(src_str)
            if src_path.is_absolute():
                ctx.logger.error(
                    f"{MAGENTA}{ctx.logger.fmt_path(link_toml)}{RESET}: "
                    f"[[link]] src must be relative, got {repr(src_str)}"
                )
                sys.exit(1)

            resolved_src = (directory / src_path).resolve()
            if not resolved_src.exists():
                ctx.logger.error(
                    f"{MAGENTA}{ctx.logger.fmt_path(link_toml)}{RESET}: "
                    f"[[link]] src {repr(src_str)} does not exist"
                )
                sys.exit(1)

        entry_dst_str: str | None = entry.get("dst")
        entry_dst = Path(entry_dst_str).expanduser() if entry_dst_str else None

        entry_linker_str: str = entry.get("linker", LinkerType.SYMLINK.value)
        try:
            entry_linker = LinkerType(entry_linker_str)
        except ValueError:
            ctx.logger.error(
                f"{MAGENTA}{ctx.logger.fmt_path(link_toml)}{RESET}: "
                f"[[link]] unknown linker {repr(entry_linker_str)}"
            )
            sys.exit(1)

        raw_tags = entry.get("tags", [])
        if not isinstance(raw_tags, list) or not all(
            isinstance(t, str) for t in raw_tags
        ):
            ctx.logger.error(
                f"{MAGENTA}{ctx.logger.fmt_path(link_toml)}{RESET}: "
                "[[link]] 'tags' must be a list of strings"
            )
            sys.exit(1)
        entry_tags: tuple[str, ...] = tuple(raw_tags)

        links.append(
            Link(
                src=resolved_src,
                dst=entry_dst,
                linker=entry_linker,
                tags=entry_tags,
            )
        )

    return DirConfig(links=links)


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


# ---------------------------------------------------------------------------
# Windows helpers
# ---------------------------------------------------------------------------


def _win_path_from_mnt(path: Path) -> str | None:
    """Convert /mnt/X/... to X:\\... Windows path, or None if not a /mnt/ path."""
    parts = path.parts
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


# ---------------------------------------------------------------------------
# Linker implementations
# ---------------------------------------------------------------------------


class IgnoreLinker(Linker):
    name = LinkerType.IGNORE

    def link(self, link: Link, ctx: Context) -> LinkerResult:
        return LinkerDelegate(links=[])

    def unlink(self, link: Link, ctx: Context) -> LinkerResult:
        return LinkerDelegate(links=[])


class SymlinkLinker(Linker):
    name = LinkerType.SYMLINK

    def link(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        src_abs = link.src.resolve()
        dst = link.dst

        if dst.is_symlink():
            actual = dst.resolve()
            if actual == src_abs:
                return LinkerSuccess(changed=False)
            msg = (
                f"{RED}{ctx.logger.fmt_path(dst)}{RESET}: symlink exists but points to wrong file.\n"
                f"{GRAY}{INDENT}current:  {ctx.logger.fmt_path(actual)}\n"
                f"{GRAY}{INDENT}expected: {ctx.logger.fmt_path(src_abs)}{RESET}"
            )
            if not ctx.force:
                return LinkerError(
                    msg + f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                )
            if not ctx.dry_run:
                dst.unlink()
                dst.symlink_to(src_abs)
            return LinkerSuccess(forced=True)

        if dst.exists():
            msg = f"{RED}{ctx.logger.fmt_path(dst)}{RESET}: exists and is not a symlink"
            if dst.is_dir():
                return LinkerError(
                    msg + f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                )
            if not ctx.force:
                return LinkerError(
                    msg + f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                )
            if not ctx.dry_run:
                dst.unlink()
                dst.parent.mkdir(parents=True, exist_ok=True)
                dst.symlink_to(src_abs)
            return LinkerSuccess(forced=True)

        if not ctx.dry_run:
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.symlink_to(src_abs)
        return LinkerSuccess()

    def unlink(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        dst = link.dst
        if dst.is_symlink():
            actual = dst.resolve()
            expected = link.src.resolve()
            if actual != expected:
                msg = (
                    f"{RED}{ctx.logger.fmt_path(dst)}{RESET}: symlink points to wrong file — skipping.\n"
                    f"{GRAY}{INDENT}current:  {ctx.logger.fmt_path(actual)}\n"
                    f"{GRAY}{INDENT}expected: {ctx.logger.fmt_path(expected)}{RESET}"
                )
                if not ctx.force:
                    return LinkerError(
                        msg
                        + f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                    )
            if not ctx.dry_run:
                dst.unlink()
            return LinkerSuccess()
        elif dst.exists():
            return LinkerError(
                f"{RED}{ctx.logger.fmt_path(dst)}{RESET}: exists and is not a symlink"
                f" — cannot unlink\n{GRAY}{INDENT}you must solve this manually{RESET}"
            )
        else:
            return LinkerSuccess(changed=False)


class CopyLinker(Linker):
    name = LinkerType.COPY

    def link(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        if link.src.is_dir() and not link.src.is_symlink():
            return LinkerDelegate(
                links=self._collect_links(link.src, link.dst, ctx)
            )
        return self._copy_file(link.src, link.dst, ctx)

    def unlink(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        if link.src.is_dir() and not link.src.is_symlink():
            return LinkerDelegate(
                links=self._collect_links(link.src, link.dst, ctx)
            )
        return self._uncopy_file(link.dst, ctx)

    def _collect_links(self, src: Path, dst: Path, ctx: Context) -> list[Link]:
        entries = [e for e in sorted(src.iterdir()) if e.name != LINK_TOML]
        if ctx.repo_root is not None:
            ignored = _git_ignored(entries, ctx.repo_root)
            entries = [e for e in entries if e not in ignored]
        return [
            Link(
                src=entry.resolve(),
                dst=dst / entry.name,
                linker=LinkerType.COPY,
            )
            for entry in entries
            if entry.is_dir() or entry.is_file() or entry.is_symlink()
        ]

    def _copy_file(self, src: Path, dst: Path, ctx: Context) -> LinkerResult:
        if dst.exists():
            if filecmp.cmp(src, dst, shallow=False):
                return LinkerSuccess(changed=False)
            return LinkerError(
                f"{RED}{ctx.logger.fmt_path(dst)}{RESET}: exists with different content"
                f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
            )
        if not ctx.dry_run:
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
        return LinkerSuccess()

    def _uncopy_file(self, dst: Path, ctx: Context) -> LinkerResult:
        if dst.exists():
            if not ctx.dry_run:
                dst.unlink()
            return LinkerSuccess()
        return LinkerSuccess(changed=False)


class SpreadLinker(Linker):
    """
    Symlinks each immediate child of link.src into link.dst/.

    Also the root entry-point linker: SpreadLinker().link(Link(src=home/, dst=~/), ctx).
    Per-child behaviour is determined by:
      1. [[link]] override entries in link.src/.link.toml (by matching src path)
      2. The child directory's own .link.toml primary config
      3. Default: SymlinkLinker
    Remaining [[link]] entries whose src is not a direct child are processed as
    standalone extra links after iterating children.
    """

    name = LinkerType.SPREAD

    def link(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        return LinkerDelegate(links=self._collect_child_links(link, ctx))

    def unlink(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        return LinkerDelegate(links=self._collect_child_links(link, ctx))

    def _collect_child_links(self, link: Link, ctx: Context) -> list[Link]:
        dir_config = _read_dir_config(link.src, ctx)
        dir_abs = link.src.resolve()

        # Child-override entries: those with src != the directory itself.
        overrides: dict[Path, list[Link]] = {}
        for e in dir_config.links:
            if e.src != dir_abs:
                overrides.setdefault(e.src, []).append(e)

        children = [
            c for c in sorted(link.src.iterdir()) if c.name != LINK_TOML
        ]
        if ctx.repo_root is not None:
            ignored = _git_ignored(children, ctx.repo_root)
            children = [c for c in children if c not in ignored]

        consumed: set[Path] = set()
        result: list[Link] = []

        for child in children:
            child_abs = child.resolve()
            if child_abs in overrides:
                consumed.add(child_abs)
                entries = overrides[child_abs]
                matching = [
                    e for e in entries if _link_matches_tags(e, ctx.active_tags)
                ]
                # If any entry is tagged and none match → skip (explicit env control).
                # If all entries are tagless → apply all (backward compat).
                to_apply = (
                    matching
                    if (matching or any(e.tags for e in entries))
                    else entries
                )
                for override in to_apply:
                    effective_dst = override.dst or (link.dst / child.name)
                    result.append(
                        Link(
                            src=child_abs,
                            dst=effective_dst,
                            linker=override.linker,
                        )
                    )
            elif (
                child.is_dir()
                and not child.is_symlink()
                and (child / LINK_TOML).exists()
            ):
                child_config = _read_dir_config(child, ctx)
                # No-src entries (src == child_abs) are the primary config for this child.
                primary_entries = [
                    e for e in child_config.links if e.src == child_abs
                ]
                if primary_entries:
                    to_apply = _filter_by_tags(primary_entries, ctx.active_tags)
                    for p in to_apply:
                        effective_dst = p.dst or (link.dst / child.name)
                        result.append(
                            Link(
                                src=child_abs,
                                dst=effective_dst,
                                linker=p.linker,
                            )
                        )
                else:
                    # No primary defined → default symlink.
                    result.append(
                        Link(
                            src=child_abs,
                            dst=link.dst / child.name,
                            linker=LinkerType.SYMLINK,
                        )
                    )
            else:
                result.append(
                    Link(
                        src=child_abs,
                        dst=link.dst / child.name,
                        linker=LinkerType.SYMLINK,
                    )
                )

        # Process extra [[link]] entries whose src was not a direct child
        for src_path, entries in overrides.items():
            if src_path in consumed:
                continue
            to_apply = _filter_by_tags(entries, ctx.active_tags)
            for extra in to_apply:
                effective_dst = extra.dst or (link.dst / src_path.name)
                result.append(
                    Link(src=src_path, dst=effective_dst, linker=extra.linker)
                )

        return result


class RecursiveLinker(Linker):
    """
    Recurses deeply into link.src, creating symlinks for leaf files.
    Stops recursing into a subdirectory when that subdirectory has a .link.toml,
    dispatching to its configured linker instead.
    """

    name = LinkerType.RECURSIVE

    def link(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        return LinkerDelegate(links=self._collect_child_links(link, ctx))

    def unlink(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        dst = link.dst

        def cleanup() -> None:
            if not ctx.dry_run and dst.is_dir() and not dst.is_symlink():
                try:
                    dst.rmdir()
                except OSError:
                    pass

        return LinkerDelegate(
            links=self._collect_child_links(link, ctx), post_action=cleanup
        )

    def _collect_child_links(self, link: Link, ctx: Context) -> list[Link]:
        children = [
            c for c in sorted(link.src.iterdir()) if c.name != LINK_TOML
        ]
        if ctx.repo_root is not None:
            ignored = _git_ignored(children, ctx.repo_root)
            children = [c for c in children if c not in ignored]

        result = []
        for child in children:
            child_dst = link.dst / child.name  # type: ignore[operator]
            if child.is_dir() and not child.is_symlink():
                child_toml = child / LINK_TOML
                if child_toml.exists():
                    child_abs = child.resolve()
                    child_config = _read_dir_config(child, ctx)
                    primary_entries = [
                        e for e in child_config.links if e.src == child_abs
                    ]
                    if primary_entries:
                        for p in _filter_by_tags(
                            primary_entries, ctx.active_tags
                        ):
                            effective_dst = p.dst or child_dst
                            result.append(
                                Link(
                                    src=child_abs,
                                    dst=effective_dst,
                                    linker=p.linker,
                                )
                            )
                    else:
                        result.append(
                            Link(
                                src=child.resolve(),
                                dst=child_dst,
                                linker=LinkerType.SYMLINK,
                            )
                        )
                else:
                    result.append(
                        Link(
                            src=child.resolve(),
                            dst=child_dst,
                            linker=LinkerType.RECURSIVE,
                        )
                    )
            elif child.is_file() or child.is_symlink():
                result.append(
                    Link(
                        src=child.resolve(),
                        dst=child_dst,
                        linker=LinkerType.SYMLINK,
                    )
                )
        return result


class WindowsSymlinkLinker(Linker):
    """
    Creates a Windows junction/symlink from a Windows path to the WSL source.

    Required when dst is on the Windows filesystem (/mnt/<drive>/...), because
    regular WSL symlinks are not recognised by Windows applications.
    Uses elevated PowerShell (New-Item -ItemType SymbolicLink).
    """

    name = LinkerType.WINDOWS_SYMLINK

    def link(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        win_dst = _win_path_from_mnt(link.dst)
        if win_dst is None:
            return LinkerError(
                f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: windows-symlink requires "
                f"a /mnt/<drive>/... destination"
                f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
            )
        src_abs = link.src.resolve()
        unc_src = _wsl_to_unc(src_abs)

        ps_result = _run_ps(
            f'$i = Get-Item "{win_dst}" -ErrorAction SilentlyContinue\n'
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
            return LinkerSuccess(changed=False)

        if rc == 3:
            if not ctx.force:
                return LinkerError(
                    f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: exists on Windows and is not a symlink"
                    + (
                        f"\n{GRAY}{INDENT}--force provided, overriding to "
                        f"{MAGENTA}{ARROW} {ctx.logger.fmt_path(src_abs)}{RESET}"
                        if ctx.force
                        else f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                    )
                )
            if not ctx.dry_run:
                ctx.logger.notice(
                    f"(requesting Windows administrator access to remove {win_dst}"
                    " — a UAC prompt will appear)"
                )
                rm_result = _run_ps_elevated(
                    f'Remove-Item "{win_dst}" -Force -Recurse'
                )
                if rm_result.returncode != 0:
                    return LinkerError(
                        f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: "
                        f"failed to remove existing Windows item"
                        f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                    )
        elif rc == 1:
            actual_win = ps_result.stdout.decode(errors="replace").strip()
            if not ctx.force:
                return LinkerError(
                    f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: "
                    "Windows symlink exists but points to wrong file.\n"
                    f"{GRAY}{INDENT}current:  {actual_win}\n"
                    f"{GRAY}{INDENT}expected: {unc_src}{RESET}"
                    f"\n{GRAY}{INDENT}(use --force to override){RESET}"
                )
            if not ctx.dry_run:
                ctx.logger.notice(
                    f"(requesting Windows administrator access to remove {win_dst}"
                    " — a UAC prompt will appear)"
                )
                rm_result = _run_ps_elevated(f'Remove-Item "{win_dst}" -Force')
                if rm_result.returncode != 0:
                    return LinkerError(
                        f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: "
                        f"failed to remove existing Windows symlink"
                        f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                    )

        # rc == 2 (absent) or force-cleared rc == 1/3: create the symlink
        if not ctx.dry_run:
            ctx.logger.notice(
                f"(requesting Windows administrator access to create symlink at {win_dst}"
                " — a UAC prompt will appear)"
            )
            create_result = _run_ps_elevated(
                f'New-Item -ItemType SymbolicLink -Path "{win_dst}" -Target "{unc_src}" -Force'
            )
            if create_result.returncode != 0:
                return LinkerError(
                    f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: "
                    f"failed to create Windows symlink"
                    f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                )
        return LinkerSuccess(forced=(rc != 2))

    def unlink(self, link: Link, ctx: Context) -> LinkerResult:
        assert link.dst is not None
        win_dst = _win_path_from_mnt(link.dst)
        if win_dst is None:
            return LinkerError(
                f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: windows-symlink requires "
                f"a /mnt/<drive>/... destination"
                f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
            )

        if (
            _run_ps(
                f'if (Test-Path "{win_dst}") {{ exit 0 }} else {{ exit 1 }}'
            ).returncode
            != 0
        ):
            return LinkerSuccess(changed=False)

        if not ctx.dry_run:
            ctx.logger.notice(
                f"(requesting Windows administrator access to remove {win_dst}"
                " — a UAC prompt will appear)"
            )
            rm_result = _run_ps_elevated(f'Remove-Item "{win_dst}" -Force')
            if rm_result.returncode != 0:
                return LinkerError(
                    f"{RED}{ctx.logger.fmt_path(link.dst)}{RESET}: "
                    f"failed to remove Windows symlink"
                    f"\n{GRAY}{INDENT}you must solve this manually{RESET}"
                )
        return LinkerSuccess()


LINKERS: dict[LinkerType, Linker] = {
    LinkerType.SYMLINK: SymlinkLinker(),
    LinkerType.SPREAD: SpreadLinker(),
    LinkerType.RECURSIVE: RecursiveLinker(),
    LinkerType.COPY: CopyLinker(),
    LinkerType.WINDOWS_SYMLINK: WindowsSymlinkLinker(),
    LinkerType.IGNORE: IgnoreLinker(),
}


# ---------------------------------------------------------------------------
# Orchestrator
# ---------------------------------------------------------------------------


def dispatch(link: Link, ctx: Context, *, unlink: bool = False) -> None:
    """Dispatch a link to its linker, handle the result, and recurse as needed."""
    assert link.dst is not None
    impl = LINKERS[link.linker]
    result: LinkerResult = (
        impl.unlink(link, ctx) if unlink else impl.link(link, ctx)
    )

    if isinstance(result, LinkerDelegate):
        for sub_link in result.links:
            dispatch(sub_link, ctx, unlink=unlink)
        if result.post_action is not None:
            result.post_action()

    elif isinstance(result, LinkerSuccess):
        if not unlink:
            if not ctx.dry_run:
                ctx.state.add(link)
            if result.forced:
                ctx.logger.forced(link.dst, link.src)
            elif result.changed:
                ctx.logger.link(link.dst, link.src)
            else:
                ctx.logger.exists(link.dst)
        else:
            if result.changed:
                if not ctx.dry_run:
                    ctx.state.remove(link.dst)
                ctx.logger.unlinked(link.dst)
            else:
                ctx.logger.absent(link.dst)

    elif isinstance(result, LinkerError):
        ctx.logger.error(result.message)
        ctx.had_conflict = True


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


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
        help="Remove managed links instead of creating them.",
    )
    p.add_argument(
        "--force",
        action="store_true",
        help="Overwrite conflicting files/symlinks instead of aborting.",
    )
    p.add_argument(
        "--tags",
        metavar="TAG",
        nargs="*",
        help=(
            "Override active tags (space-separated). Skips auto-detection. "
            "Useful with --dry-run to simulate a specific environment."
        ),
    )
    args = p.parse_args()

    tags_toml = PROJECT_ROOT / "dev" / TAGS_TOML
    active_tags = (
        frozenset(args.tags)
        if args.tags is not None
        else load_active_tags(tags_toml)
    )

    logger = Logger(verbose=args.verbose)
    if active_tags:
        logger.notice(f"active tags: {', '.join(sorted(active_tags))}")
    state = LinksState(PROJECT_ROOT / LINKS_STATE)
    state.load()
    ctx = Context(
        dry_run=args.dry_run,
        force=args.force,
        logger=logger,
        state=state,
        repo_root=PROJECT_ROOT,
        active_tags=active_tags,
    )

    all_link_tomls = set(config_dir.rglob(LINK_TOML))

    if args.dry_run:
        logger.notice("(--dry-run: no changes will be made)")

    root_link = Link(src=config_dir, dst=Path.home(), linker=LinkerType.SPREAD)

    if args.unlink:
        dispatch(root_link, ctx, unlink=True)
        # Clean up stale state entries (src removed from repo since last run)
        for state_link in state.items():
            assert state_link.dst is not None
            dispatch(state_link, ctx, unlink=True)
    else:
        dispatch(root_link, ctx)
        # Remove orphaned links: src deleted from repo, dst still exists
        for state_link in state.items():
            assert state_link.dst is not None
            if not state_link.src.exists():
                if not args.dry_run and state_link.dst.is_symlink():
                    state_link.dst.unlink()
                    state.remove(state_link.dst)
                logger.removed_orphan(state_link.dst, state_link.src)

    # Warn about .link.toml files that were never consulted
    unvisited = all_link_tomls - ctx._visited_link_tomls
    for path in sorted(unvisited):
        logger.notice(
            f"warning: {MAGENTA}{logger.fmt_path(path)}{YELLOW} was never read "
            f"(shadowed by a parent linker){RESET}"
        )

    if args.dry_run:
        logger.notice("(--dry-run: no changes have been made)")

    if not args.unlink:
        print(f"{GREEN}✓ {logger.target_count} targets linked{RESET}")

    if not args.dry_run:
        state.save()

    if ctx.had_conflict:
        if args.force:
            logger.error(
                f"{RED}some conflicts must be resolved manually{RESET}"
            )
        else:
            logger.error(f"{RED}conflicts found{RESET}")
        sys.exit(1)


if __name__ == "__main__":
    main()
