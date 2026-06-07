#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Claude Code UserPromptSubmit hook.

Before each agent turn, snapshot the current jj operation id (and the prompt
that started the turn) so the turn can be rolled back with `jj op restore <id>`.

Appends a TOML entry to a file mirroring the workspace path under
~/.local/share/agent-jj-op-ids/claude/, with $HOME shown as a literal "~"
(e.g. a session in $HOME/workspaces/foo -> .../claude/~/workspaces/foo).
Keeps at most MAX_ENTRIES newest entries per workspace.
"""

import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path

BASE = Path.home() / ".local/share/agent-jj-op-ids/claude"
MAX_ENTRIES = 100


def current_jj_op_id(cwd: str) -> str | None:
    """Return the short current jj operation id, or None if cwd isn't a jj repo."""
    try:
        r = subprocess.run(
            [
                "jj",
                "op",
                "log",
                "--no-graph",
                "--limit",
                "1",
                "-T",
                "id.short()",
            ],
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    op_id = r.stdout.strip()
    return op_id if r.returncode == 0 and op_id else None


def workspace_file(cwd: str) -> Path:
    """Map a workspace dir to its snapshot file, mirroring the path ($HOME -> ~)."""
    home = str(Path.home())
    if cwd == home:
        rel = "~"
    elif cwd.startswith(home + "/"):
        rel = "~" + cwd[len(home) :]
    else:
        rel = cwd.lstrip("/")
    return BASE / f"{rel}.toml"


def load_prompts(path: Path) -> list[dict[str, str]]:
    """Read existing [[prompts]] entries; tolerate a missing or corrupt file."""
    if not path.exists():
        return []
    try:
        with path.open("rb") as f:
            entries = tomllib.load(f).get("prompts", [])
    except (OSError, tomllib.TOMLDecodeError):
        return []
    return [
        {
            "jj_op_id": str(e.get("jj_op_id", "")),
            "user_prompt": str(e.get("user_prompt", "")),
        }
        for e in entries
    ]


def render(workspace: str, prompts: list[dict[str, str]]) -> str:
    """Render the TOML doc. json.dumps yields valid TOML basic strings."""
    lines = [f"workspace = {json.dumps(workspace)}", ""]
    for p in prompts:
        lines += [
            "[[prompts]]",
            f"jj_op_id = {json.dumps(p.get('jj_op_id', ''))}",
            f"user_prompt = {json.dumps(p.get('user_prompt', ''))}",
            "",
        ]
    return "\n".join(lines) + "\n"


def main() -> None:
    try:
        event = json.loads(sys.stdin.read())
    except (json.JSONDecodeError, ValueError):
        return
    cwd = event.get("cwd") or str(Path.cwd())
    prompt = event.get("prompt")
    if not isinstance(prompt, str):
        prompt = ""

    op_id = current_jj_op_id(cwd)
    if op_id is None:  # not a jj repo (or jj unavailable) — nothing to snapshot
        return

    path = workspace_file(cwd)
    path.parent.mkdir(parents=True, exist_ok=True)

    prompts = load_prompts(path)
    prompts.append({"jj_op_id": op_id, "user_prompt": prompt})
    prompts = prompts[-MAX_ENTRIES:]

    # Atomic write so concurrent sessions in the same workspace can't corrupt it.
    tmp = path.with_name(f"{path.name}.tmp.{os.getpid()}")
    tmp.write_text(render(cwd, prompts))
    os.replace(tmp, path)


if __name__ == "__main__":
    main()
