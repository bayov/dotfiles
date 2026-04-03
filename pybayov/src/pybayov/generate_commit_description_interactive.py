#!/usr/bin/env python3
"""
Generates a commit description interactively using AI.

Uses `generate_commit_description` to get the description, and shows a
spinner box while waiting.

If a description already exists for the current commit, prompts the user to
confirm if they want to override it before generating a new one.

Finally, uses `jj describe` to set the generated description for the current commit.
"""

import subprocess
import sys

from pybayov import echo_box as _eb
from pybayov import generate_commit_description as _gcd
from pybayov import spinner as _spinner


def run(cmd, **kwargs):
    return subprocess.run(cmd, **kwargs)


def main() -> None:
    r = run(["fish", "-c", "precommit"])
    if r.returncode != 0:
        sys.exit(r.returncode)

    r = run(
        ["jj", "log", "-r", "@", "-n", "1", "--no-graph", "-T", "description"],
        capture_output=True,
        text=True,
    )
    current_desc = r.stdout.strip()

    if current_desc:
        inner_width = (
            _eb.DEFAULT_WIDTH - _eb.DEFAULT_PAD_LEFT - _eb.DEFAULT_PAD_RIGHT
        )
        lines = [
            w
            for raw in current_desc.splitlines() or [""]
            for w in _eb.word_wrap(raw, inner_width)
        ]
        print(
            _eb.render_box(
                lines,
                _eb.DEFAULT_WIDTH,
                _eb.DEFAULT_BG,
                margin_top=_eb.DEFAULT_MARGIN,
                margin_right=_eb.DEFAULT_MARGIN,
                margin_bottom=0,
                margin_left=_eb.DEFAULT_MARGIN,
                pad_top=_eb.DEFAULT_PAD_TOP,
                pad_right=_eb.DEFAULT_PAD_RIGHT,
                pad_bottom=_eb.DEFAULT_PAD_BOTTOM,
                pad_left=_eb.DEFAULT_PAD_LEFT,
                title="Existing Commit Description",
                title_color=3,
                title_bg=_eb.lighten(_eb.DEFAULT_BG),
            )
        )
        r = run(
            [
                "gum",
                "confirm",
                "Override description?",
                "--prompt.foreground=3",
                "--selected.background=3",
                "--padding=0 1",
            ]
        )
        if r.returncode != 0:
            sys.exit(r.returncode)

    def _generate():
        data = _gcd.generate()
        return data["commit_description"]

    try:
        description = _spinner.run_with_spinner(
            _generate,
            title="generating",
            box=True,
            box_title="Commit Description",
        )
    except _gcd.GenerateError as e:
        sys.stderr.write(f"{e}\n")
        sys.exit(1)

    if not description:
        sys.stderr.write("Empty commit description\n")
        sys.exit(1)

    run(["jj", "describe", "-m", description, "--quiet"])


if __name__ == "__main__":
    main()
