#!/usr/bin/env python3
"""Clone haka into ~/workspaces/<name> and run engine_service bootstrap.

Usage:
    new_workspace.py <name>             # clone + copy .env + generate + install
    new_workspace.py <name> --no-setup  # clone + copy only

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

SOURCE_REPO = Path.home() / "projects" / "haka"
WORKSPACES_DIR = Path.home() / "workspaces"
COPY_ALLOWLIST: list[str] = [".env"]

# Ensure Go-installed binaries (protoc-gen-go, air, etc.) are on PATH for all
# child processes. The user's fish shell exports ~/go/bin, but a bash/sh
# subprocess doesn't inherit fish-specific config.
_GOBIN = str(Path.home() / "go" / "bin")
if _GOBIN not in os.environ.get("PATH", "").split(":"):
    os.environ["PATH"] = f"{_GOBIN}:{os.environ.get('PATH', '')}"


def run(cmd: list[str], cwd: Path | None = None) -> None:
    print(f"$ {' '.join(cmd)}" + (f"  (cwd={cwd})" if cwd else ""))
    subprocess.run(cmd, cwd=cwd, check=True)


def main() -> int:
    p = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument("name", help="workspace name → ~/workspaces/<name>")
    p.add_argument(
        "--no-setup", action="store_true", help="skip just install / setup-air"
    )
    args = p.parse_args()

    dest = WORKSPACES_DIR / args.name
    if dest.exists():
        print(f"error: workspace already exists: {dest}", file=sys.stderr)
        return 1
    if not SOURCE_REPO.is_dir():
        print(f"error: source repo not found: {SOURCE_REPO}", file=sys.stderr)
        return 1

    url = subprocess.check_output(
        ["git", "-C", str(SOURCE_REPO), "remote", "get-url", "origin"],
        text=True,
    ).strip()

    WORKSPACES_DIR.mkdir(parents=True, exist_ok=True)
    run(["jj", "git", "clone", "--colocate", url, str(dest)])

    for item in COPY_ALLOWLIST:
        src = SOURCE_REPO / item
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
        engine_dir = dest / "engine" / "engine_service"
        if not engine_dir.is_dir():
            print(
                f"warning: {engine_dir} not found; skipping engine setup",
                file=sys.stderr,
            )
        else:
            # Generated Go proto/Triton-stub packages are imported by
            # engine_service; without them `go mod tidy` fails. Pass `true`
            # to force regeneration — the recipe's existence check looks at
            # `go-shared-protos/` (which is committed) instead of the
            # `generated/` subdir (which is gitignored), so the default
            # branch silently skips on a fresh clone.
            run(["just", "generate-proto", "true"], cwd=engine_dir)
            # detection_labels.go is generated from shared_consts and is
            # gitignored; engine_service won't compile without it.
            run(["just", "clients", "generate-detection-labels"], cwd=dest)
            run(["just", "install"], cwd=engine_dir)

    print(f"\nworkspace ready: {dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
