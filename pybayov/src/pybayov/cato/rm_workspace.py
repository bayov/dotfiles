#!/usr/bin/env python3
"""
Move a workspace from ~/workspaces/<name> to /tmp/deleted-workspaces/<name>.

Usage:
    rm_workspace.py [name]   # explicit workspace name
    rm_workspace.py          # infer from current directory if inside ~/workspaces

Warns and prompts before removing if the workspace has uncommitted changes
or commits not yet merged to trunk.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from pathlib import Path

WORKSPACES_DIR = Path.home() / "workspaces"
TRASH_DIR = Path("/tmp/deleted-workspaces")


def _jj(args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["jj"] + args, cwd=cwd, capture_output=True, text=True
    )


def workspace_issues(path: Path) -> list[str]:
    issues: list[str] = []

    r = _jj(["diff", "--summary"], path)
    if r.returncode == 0 and r.stdout.strip():
        issues.append("uncommitted changes in working copy")

    # Non-empty commits between trunk and working-copy parent
    r = _jj(
        [
            "log",
            "--no-graph",
            "-r",
            "trunk()..@- & ~empty()",
            "-T",
            "change_id",
        ],
        path,
    )
    if r.returncode == 0 and r.stdout.strip():
        issues.append("commits not merged to trunk")

    return issues


def main() -> int:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "name", nargs="?", help="workspace name; inferred from cwd if omitted"
    )
    args = p.parse_args()

    if args.name:
        workspace = WORKSPACES_DIR / args.name
    else:
        try:
            rel = Path.cwd().resolve().relative_to(WORKSPACES_DIR.resolve())
        except ValueError:
            print(
                f"error: not inside {WORKSPACES_DIR}; specify a workspace name",
                file=sys.stderr,
            )
            return 1
        workspace = WORKSPACES_DIR / rel.parts[0]

    if not workspace.is_dir():
        print(f"error: workspace not found: {workspace}", file=sys.stderr)
        return 1

    issues = workspace_issues(workspace)
    if issues:
        print(f"Warning: workspace '{workspace.name}' is not clean:")
        for issue in issues:
            print(f"  - {issue}")
        try:
            answer = input("Delete anyway? [Y/n] ").strip().lower()
        except (EOFError, KeyboardInterrupt):
            print("\nAborted.")
            return 1
        if answer not in ("", "y"):
            print("Aborted.")
            return 0

    TRASH_DIR.mkdir(parents=True, exist_ok=True)
    dest = TRASH_DIR / workspace.name
    if dest.exists():
        print(f"error: already exists in trash: {dest}", file=sys.stderr)
        return 1
    shutil.move(str(workspace), dest)
    print(f"moved: {workspace} → {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
