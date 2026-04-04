#!/usr/bin/env python3
"""Generates an AI commit description for the current jj commit and prints it to stdout.

With --interactive: shows a spinner, prompts before overriding an existing description,
and applies the result via `jj describe`.
"""

import json
import os
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path
from random import randint

from pybayov import echo_box as _eb
from pybayov import quick_prompt as _qp
from pybayov import spinner as _spinner

DOTFILES_DIR: Path = Path(
    os.environ.get("DOTFILES_DIR", str(Path.home() / "dotfiles"))
)


class GenerateError(Exception):
    def __init__(self, error_info: dict[str, str]):
        self.error_info = error_info
        super().__init__(error_info.get("error", "Unknown error"))


def generate() -> dict[str, str]:
    """Generate a commit description and return the result dict.

    Raises GenerateError on failure.
    """
    # 1. Get diff
    diff_result = subprocess.run(
        ["jj", "diff", "--git"], capture_output=True, text=True
    )
    diff = diff_result.stdout.strip()
    if not diff:
        raise GenerateError({"error": "No changes found."})

    # 2. Load prompt template
    template_path = Path(
        os.environ.get(
            "generate_commit_description_template",
            str(
                DOTFILES_DIR / "config/generate-commit-description-template.md"
            ),
        )
    )
    prompt = template_path.read_text().replace("{{diff}}", diff)

    # 3. Save prompt to temp file (useful for debugging)
    temp_dir = Path("/tmp/generate-commit-description")
    temp_dir.mkdir(parents=True, exist_ok=True)
    ts = datetime.now().strftime("%Y-%m-%d.%H-%M-%S")
    rid = randint(0, 999999)
    input_file = temp_dir / f"{ts}.{rid}.prompt_input.md"
    output_file = temp_dir / f"{ts}.{rid}.prompt_output.md"
    input_file.write_text(prompt)

    # 4. Run quick-prompt
    try:
        output_text = _qp.run(prompt)
    except RuntimeError as e:
        raise GenerateError({"error": f"quick-prompt failed: {e}"})
    if not output_text.strip():
        raise GenerateError({"error": "quick-prompt returned empty output"})
    output_file.write_text(output_text)

    # 5. Extract commit description from <commit-description> tags
    match = re.search(
        r"<commit-description>\s*(.*?)\s*</commit-description>",
        output_text,
        re.DOTALL,
    )
    if not match:
        raise GenerateError(
            {
                "error": "Failed to extract commit description from prompt output.",
                "prompt_input_file": str(input_file),
                "prompt_output_file": str(output_file),
            }
        )

    commit_description = match.group(1).strip()
    output_file.write_text(commit_description)

    return {
        "prompt_input_file": str(input_file),
        "prompt_output_file": str(output_file),
        "commit_description": commit_description,
    }


def _interactive_main() -> None:
    precommit_result: subprocess.CompletedProcess[bytes] = subprocess.run(
        ["fish", "-c", "precommit"]
    )
    if precommit_result.returncode != 0:
        sys.exit(precommit_result.returncode)

    log_result = subprocess.run(
        ["jj", "log", "-r", "@", "-n", "1", "--no-graph", "-T", "description"],
        capture_output=True,
        text=True,
    )
    current_desc: str = log_result.stdout.strip()

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
                _eb.BoxConfig(
                    margin_top=_eb.DEFAULT_MARGIN,
                    margin_right=_eb.DEFAULT_MARGIN,
                    margin_bottom=0,
                    margin_left=_eb.DEFAULT_MARGIN,
                    title="Existing Commit Description",
                    title_color=3,
                    title_bg=_eb.lighten(_eb.DEFAULT_BG),
                ),
            )
        )
        confirm_result: subprocess.CompletedProcess[bytes] = subprocess.run(
            [
                "gum",
                "confirm",
                "Override description?",
                "--prompt.foreground=3",
                "--selected.background=3",
                "--padding=0 1",
            ]
        )
        if confirm_result.returncode != 0:
            sys.exit(confirm_result.returncode)

    try:
        description = _spinner.run_with_spinner(
            lambda: generate()["commit_description"],
            title="generating commit description...",
            box=True,
            box_title="Commit Description",
        )
    except GenerateError as e:
        sys.stderr.write(f"{e}\n")
        sys.exit(1)

    if not description:
        sys.stderr.write("Empty commit description\n")
        sys.exit(1)

    subprocess.run(["jj", "describe", "-m", description, "--quiet"])


def main() -> None:
    if "--interactive" in sys.argv:
        _interactive_main()
        return
    try:
        data = generate()
    except GenerateError as e:
        print(json.dumps(e.error_info))
        sys.exit(1)
    print(json.dumps(data, indent=2))


if __name__ == "__main__":
    main()
