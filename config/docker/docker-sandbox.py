#!/usr/bin/env python3
"""
Run a command or shell in a general-purpose Docker sandbox.
"""

import os
import subprocess
import sys
import shlex
import argparse
import uuid
import hashlib
from pathlib import Path
from dataclasses import dataclass
from datetime import datetime, timezone

# ==============================================================================
# CONFIGURATION
# ==============================================================================

IMAGE_TAG = "sandbox-env"
DOTFILES_DIR = Path.home() / "dotfiles"
DOCKERFILE_PATH = DOTFILES_DIR / "config" / "docker" / "Dockerfile.sandbox"
HASH_LABEL = "com.docker-sandbox.dockerfile-hash"


@dataclass
class Mount:
    host_path: str
    container_path: str
    read_only: bool


# Volumes to mount
# Paths starting with ~ will be expanded to the host user's home directory.
MOUNTS = [
    # The current project workspace.
    Mount(".", "/workspace", read_only=False),
    # Claude Code settings (must be writable as CC modifies these).
    Mount("~/.claude", "/home/sandbox/.claude", read_only=False),
    Mount("~/.claude.json", "/home/sandbox/.claude.json", read_only=False),
    # Gemini CLI settings (must be writable as Gemini modifies these).
    Mount("~/.gemini", "/home/sandbox/.gemini", read_only=False),
    Mount("~/.gitconfig", "/home/sandbox/.gitconfig", read_only=True),
    # Map dotfiles to the exact same path in the container to preserve symlinks
    Mount("~/dotfiles", str(DOTFILES_DIR), read_only=True),
]

# Essential terminal configurations to map from ~/.config/ -> /home/sandbox/.config/
CONFIG_MOUNTS = [
    "fish",
    "starship.toml",
    "jj",
    "delta",
    "eza",
    "fresh",
    "quick_prompt_models.json",
    "claude-code",
]

# Additional optional directories to map if they exist
OPTIONAL_MOUNTS = [
    Mount(
        "~/.local/share/quick_prompt",
        "/home/sandbox/.local/share/quick_prompt",
        read_only=False,
    ),
]

# Environment variables to forward from host to container
ENV_VARS_TO_FORWARD = [
    "TERM",
    "COLORTERM",
    "LANG",
    "LC_ALL",
    "FORCE_COLOR",
]

# ==============================================================================
# SCRIPT LOGIC
# ==============================================================================


def expand_path(p: str) -> Path:
    if p == ".":
        return Path.cwd().resolve()
    return Path(p).expanduser().resolve()


def setup_history_dir() -> Path:
    """Setup a directory on the host to store command history from the sandbox."""
    now = datetime.now()
    date_str = now.strftime("%Y-%m-%d")
    time_str = now.strftime("%H-%M-%S")
    unique_id = uuid.uuid4().hex[:8]
    cwd_name = Path.cwd().name

    history_base = (
        Path.home() / ".local" / "share" / "docker-sandbox" / "command_history"
    )
    history_dir = history_base / date_str / f"{time_str}.{unique_id}.{cwd_name}"

    history_dir.mkdir(parents=True, exist_ok=True)

    # Pre-create files to avoid Docker creating them as directories if we mount them individually
    (history_dir / "bash_history.txt").touch()
    (history_dir / "fish_history.txt").touch()

    return history_dir


def get_dockerfile_hash() -> str:
    """Calculate the SHA256 hash of the Dockerfile."""
    with open(DOCKERFILE_PATH, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def build_image(rebuild: bool = False):
    """Ensure the sandbox Docker image is built and up to date."""
    if not DOCKERFILE_PATH.exists():
        print(
            f"\033[31mError: Dockerfile not found at {DOCKERFILE_PATH}\033[0m",
            file=sys.stderr,
        )
        sys.exit(1)

    current_hash = get_dockerfile_hash()

    # Check image status and its stored hash label
    inspect_cmd = [
        "docker",
        "image",
        "inspect",
        IMAGE_TAG,
        "--format",
        f'{{{{index .Config.Labels "{HASH_LABEL}"}}}}',
    ]
    result = subprocess.run(inspect_cmd, capture_output=True, text=True)

    if rebuild:
        print(
            f"\033[33mRebuilding sandbox image ({IMAGE_TAG}) as requested...\033[0m"
        )
    elif result.returncode != 0:
        print(
            f"\033[33mBuilding sandbox image ({IMAGE_TAG}) for the first time...\033[0m"
        )
    else:
        # Image exists, check if hash matches
        image_hash = result.stdout.strip()
        if image_hash == current_hash:
            return  # Hash matches, no need to rebuild
        else:
            print(
                f"\033[33mDockerfile changed. Rebuilding sandbox image ({IMAGE_TAG})...\033[0m"
            )

    build_cmd = [
        "docker",
        "build",
        "-t",
        IMAGE_TAG,
        "--label",
        f"{HASH_LABEL}={current_hash}",
        "-f",
        str(DOCKERFILE_PATH),
        str(DOCKERFILE_PATH.parent),
    ]

    if rebuild:
        build_cmd.insert(2, "--no-cache")

    subprocess.run(build_cmd, check=True)


def get_mount_args():
    args = []

    # Standard Mounts
    for mount in MOUNTS:
        host_path = expand_path(mount.host_path)

        # Only mount if it exists, or if it's the workspace (.)
        if mount.host_path != "." and not host_path.exists():
            continue

        ro_flag = ":ro" if mount.read_only else ""
        args.extend(
            ["--volume", f"{host_path}:{mount.container_path}{ro_flag}"]
        )

    # Config Mounts
    for conf in CONFIG_MOUNTS:
        host_conf = Path.home() / ".config" / conf
        if host_conf.exists():
            args.extend(
                ["--volume", f"{host_conf}:/home/sandbox/.config/{conf}"]
            )

    # Optional Mounts
    for mount in OPTIONAL_MOUNTS:
        host_path = expand_path(mount.host_path)
        if host_path.exists():
            ro_flag = ":ro" if mount.read_only else ""
            args.extend(
                ["--volume", f"{host_path}:{mount.container_path}{ro_flag}"]
            )

    return args


def get_env_and_ssh_args(with_ssh=False):
    args = []

    # Pass configured environment variables
    for env_var in ENV_VARS_TO_FORWARD:
        val = os.environ.get(env_var)
        if val:
            args.extend(["--env", f"{env_var}={val}"])

    if with_ssh:
        ssh_auth_sock = os.environ.get("SSH_AUTH_SOCK")
        if ssh_auth_sock:
            args.extend(["--volume", f"{ssh_auth_sock}:/tmp/ssh-agent.sock"])
            args.extend(["--env", "SSH_AUTH_SOCK=/tmp/ssh-agent.sock"])
        else:
            print(
                "\033[33mWarning: --with-ssh was provided but SSH_AUTH_SOCK is not set.\033[0m"
            )

    return args


def main():
    parser = argparse.ArgumentParser(
        description="Run a command or shell in a general-purpose Docker sandbox."
    )
    parser.add_argument(
        "--rebuild", action="store_true", help="Rebuild the sandbox image"
    )
    parser.add_argument(
        "--with-ssh",
        action="store_true",
        help="Enable SSH agent forwarding (allows git push from within sandbox)",
    )
    parser.add_argument(
        "command",
        nargs="*",
        help="Command to run in the sandbox (default: fish shell)",
    )

    args = parser.parse_args()

    try:
        build_image(args.rebuild)
    except subprocess.CalledProcessError:
        sys.exit(1)

    history_dir = setup_history_dir()
    mount_args = get_mount_args()

    # Add history mounts
    mount_args.extend(
        [
            "--volume",
            f"{history_dir / 'bash_history.txt'}:/home/sandbox/.bash_history:rw",
            "--volume",
            f"{history_dir / 'fish_history.txt'}:/home/sandbox/.local/share/fish/fish_history:rw",
        ]
    )

    env_and_ssh_args = get_env_and_ssh_args(args.with_ssh)

    # Add Bash HISTFILE env var
    env_and_ssh_args.extend(["--env", "HISTFILE=/home/sandbox/.bash_history"])

    cmd = (
        ["docker", "run", "-it", "--rm"]
        + mount_args
        + env_and_ssh_args
        + ["--workdir", "/workspace", IMAGE_TAG]
    )

    if args.command:
        # Quote and join the arguments, as we pass them to fish -i -c
        cmd_str = " ".join(shlex.quote(a) for a in args.command)
        print(
            f"\033[33mStarting '{cmd_str}' in docker-sandbox container...\033[0m"
        )
        cmd.extend(["/usr/bin/fish", "-i", "-c", cmd_str])
    else:
        print(
            "\033[33mStarting fish shell in docker-sandbox container...\033[0m"
        )

    # Run the container, replacing current process
    os.execvp("docker", cmd)


if __name__ == "__main__":
    main()
