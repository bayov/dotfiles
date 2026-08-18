#!/usr/bin/env python3
"""
Clone a project into ~/workspaces/<name> and run its bootstrap.

Usage:
    new_workspace.py <project> <name>             # clone + copy .env + generate
    new_workspace.py <project> <name> --no-setup  # clone + copy only

Skips any step that installs globally (e.g. `just setup-air`, which puts
`air` in ~/go/bin). Run those manually once per machine if needed. The Go
module cache (~/go/pkg/mod) is shared system-wide, so `just install` is
near-instant on subsequent workspaces. Only .env (which carries secrets
and isn't checked in) is copied from the source clone.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

PROJECTS_DIR = Path.home() / "projects"
WORKSPACES_DIR = Path.home() / "workspaces"
SUPPORTED_PROJECTS: list[str] = ["haka", "frontend"]
COPY_ALLOWLIST: list[str] = [".env"]

# Ensure Go-installed binaries (protoc-gen-go, air, etc.) are on PATH for all
# child processes. The user's fish shell exports ~/go/bin, but a bash/sh
# subprocess doesn't inherit fish-specific config.
_GOBIN = str(Path.home() / "go" / "bin")
if _GOBIN not in os.environ.get("PATH", "").split(":"):
    os.environ["PATH"] = f"{_GOBIN}:{os.environ.get('PATH', '')}"


def run(cmd: list[str], cwd: Path | None = None) -> None:
    """Print and execute a shell command, raising on failure."""
    print(f"$ {' '.join(cmd)}" + (f"  (cwd={cwd})" if cwd else ""))
    subprocess.run(cmd, cwd=cwd, check=True)


def setup_haka(dest: Path) -> None:
    """
    Bootstrap a haka workspace.

    Generate protos, install engine deps, and install all components
    (Python venvs + generated clients).
    """
    engine_dir = dest / "engine" / "engine_service"
    if not engine_dir.is_dir():
        print(
            f"warning: {engine_dir} not found; skipping engine setup",
            file=sys.stderr,
        )
        return
    # Generated Go proto/Triton-stub packages are imported by engine_service;
    # without them `go mod tidy` fails. Pass `true` to force regeneration —
    # the recipe's existence check looks at `go-shared-protos/` (which is
    # committed) instead of the `generated/` subdir (which is gitignored),
    # so the default branch silently skips on a fresh clone.
    run(["just", "generate-proto", "true"], cwd=engine_dir)
    # The Go/Python const packages under */generated/ are built from
    # shared_consts/*.yaml and are gitignored, so a fresh clone has none of
    # them and `go mod tidy` fails on the missing imports (detection_labels,
    # classifier_log_types, config_sync_types, fw_origin_domain, ...). Use the
    # umbrella recipe so newly added const sets are picked up automatically.
    run(["just", "clients", "generate-consts"], cwd=dest)
    run(["just", "install"], cwd=engine_dir)
    # The engine `install` above only handles Go deps. The root `install`
    # module (justfiles/install.just -> `all`) creates the Python venvs every
    # service expects (uv sync for schema/libraries/api_server/...) and
    # generates the editable API-client packages they depend on. Without it no
    # `.venv` exists, and the IDE (pyrightconfig.json points at each service's
    # .venv site-packages) can't resolve imports.
    run(["just", "install"], cwd=dest)


def setup_frontend(dest: Path) -> None:
    """Bootstrap a frontend workspace: install pnpm deps."""
    run(["pnpm", "run", "refresh:env"], cwd=dest)


def main() -> int:
    """Parse args, clone, copy, and optionally bootstrap the workspace."""
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument(
        "project",
        choices=SUPPORTED_PROJECTS,
        help="project to clone from ~/projects/<project>",
    )
    p.add_argument("name", help="workspace name → ~/workspaces/<name>")
    p.add_argument(
        "--no-setup", action="store_true", help="skip just install / setup-air"
    )
    args = p.parse_args()

    source_repo = PROJECTS_DIR / args.project
    dest = WORKSPACES_DIR / args.name
    if dest.exists():
        print(f"error: workspace already exists: {dest}", file=sys.stderr)
        return 1
    if not source_repo.is_dir():
        print(f"error: source repo not found: {source_repo}", file=sys.stderr)
        return 1

    # Use jj (not raw git) to read the origin URL: the source repo is a
    # colocated jj repo, and a `git` wrapper on PATH rejects raw git when
    # invoked under an agent, which would kill this script before cloning.
    remotes = subprocess.check_output(
        ["jj", "--repository", str(source_repo), "git", "remote", "list"],
        text=True,
    )
    url = next(
        (
            line.split(maxsplit=1)[1]
            for line in remotes.splitlines()
            if line.split(maxsplit=1)[:1] == ["origin"]
        ),
        "",
    ).strip()
    if not url:
        print(f"error: no 'origin' remote in {source_repo}", file=sys.stderr)
        return 1

    WORKSPACES_DIR.mkdir(parents=True, exist_ok=True)
    run(["jj", "git", "clone", "--colocate", url, str(dest)])

    for item in COPY_ALLOWLIST:
        src = source_repo / item
        if not src.exists():
            print(f"skip (missing in source): {item}")
            continue
        dst = dest / item
        if src.is_dir():
            shutil.copytree(src, dst, symlinks=True)
        else:
            shutil.copy2(src, dst)
        print(f"copied: {item}")

    if not args.no_setup:
        if args.project == "haka":
            setup_haka(dest)
        elif args.project == "frontend":
            setup_frontend(dest)

    print(f"\nworkspace ready: {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
