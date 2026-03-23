#!/usr/bin/env python3
"""
Run a command or shell in a general-purpose Docker sandbox.
"""

import os
import subprocess
import sys
import shlex
import argparse
import hashlib
from pathlib import Path
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Optional

# ==============================================================================
# CONFIGURATION
# ==============================================================================

IMAGE_TAG = "sandbox-env"
DOTFILES_DIR = Path.home() / "dotfiles"
DOCKERFILE_PATH = DOTFILES_DIR / "config" / "docker" / "Dockerfile.sandbox"
HASH_LABEL = "com.docker-sandbox.dockerfile-hash"
SANDBOX_USER_HOME_DIR = "/home/sandbox"
WORKSPACE_PATH = Path.cwd().resolve()
IS_IN_DOTFILES = WORKSPACE_PATH == DOTFILES_DIR


@dataclass
class Mount:
    host_path: str
    container_path: Optional[str] = None
    ro: bool = True  # `ro` flag (read-only)

    def __post_init__(self):
        if self.container_path is None:
            if self.host_path.startswith("~"):
                self.container_path = self.host_path.replace(
                    "~", SANDBOX_USER_HOME_DIR, 1
                )
            else:
                self.container_path = self.host_path


# Volumes to mount
# Paths starting with ~ will be expanded to the host user's home directory.
MOUNTS = [
    # The current project workspace at its absolute host path.
    Mount(".", container_path=str(WORKSPACE_PATH), ro=False),
    # Claude Code settings (must be writable as CC modifies these).
    Mount("~/.claude", ro=False),
    Mount("~/.claude.json", ro=False),
    # Gemini CLI settings (must be writable as Gemini modifies these).
    Mount("~/.gemini", ro=False),
    # Map dotfiles to the exact same path in the container to preserve symlinks
    Mount(
        "~/dotfiles",
        container_path=str(DOTFILES_DIR),
        ro=not IS_IN_DOTFILES,
    ),
    # `jj` config file should be read-only, but repo configuration should be
    # writeable to allow `jj` to work in the sandbox
    Mount("~/.config/jj/config.toml", ro=not IS_IN_DOTFILES),
    Mount("~/.config/jj/repos", ro=False),
    # Fish shell writes to `fish_variables*` files
    Mount("~/.config/fish", ro=False),
    Mount("~/.config/fish/completions", ro=not IS_IN_DOTFILES),
    Mount("~/.config/fish/conf.d", ro=not IS_IN_DOTFILES),
    Mount("~/.config/fish/functions", ro=not IS_IN_DOTFILES),
    Mount("~/.config/fish/.gitignore", ro=not IS_IN_DOTFILES),
    Mount("~/.config/fish/config.fish", ro=not IS_IN_DOTFILES),
    # Other config files
    Mount("~/.gitconfig", ro=not IS_IN_DOTFILES),
    Mount("~/.config/delta", ro=not IS_IN_DOTFILES),
    Mount("~/.config/eza", ro=not IS_IN_DOTFILES),
    Mount("~/.config/fresh", ro=not IS_IN_DOTFILES),
    Mount("~/.config/quick_prompt_models.json", ro=not IS_IN_DOTFILES),
    # `quick_prompt` stats
    Mount("~/.local/share/quick_prompt", ro=False),
    # Cargo cache (registry and git dependencies)
    Mount("~/.cargo/registry", ro=False),
    Mount("~/.cargo/git", ro=False),
    # sccache cache
    Mount("~/.cache/sccache", ro=False),
]

# Environment variables to forward from host to container
ENV_VARS_TO_FORWARD = [
    "TERM",
    "COLORTERM",
    "LANG",
    "LC_ALL",
    "FORCE_COLOR",
    "SCCACHE_DIR",
    "SCCACHE_CACHE_SIZE",
    "SCCACHE_BASEDIRS",
    "RUSTC_WRAPPER",
]

# ==============================================================================
# SCRIPT LOGIC
# ==============================================================================


def expand_path(p: str) -> Path:
    if p == ".":
        return Path.cwd().resolve()
    return Path(p).expanduser().resolve()


def get_sandbox_hash() -> str:
    """Calculate a hash based on the Dockerfile and installer scripts."""
    hasher = hashlib.sha256()

    # Hash Dockerfile
    with open(DOCKERFILE_PATH, "rb") as f:
        hasher.update(f.read())

    # Hash all installation scripts
    install_scripts_dir = DOTFILES_DIR / "scripts" / "install"
    for script in sorted(install_scripts_dir.glob("*.sh")):
        with open(script, "rb") as f:
            hasher.update(script.name.encode())
            hasher.update(f.read())

    return hasher.hexdigest()


def build_image(rebuild: bool = False, no_rebuild: bool = False):
    """Ensure the sandbox Docker image is built and up to date."""
    if not DOCKERFILE_PATH.exists():
        print(
            f"\033[31mError: Dockerfile not found at {DOCKERFILE_PATH}\033[0m",
            file=sys.stderr,
        )
        sys.exit(1)

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
    image_exists = result.returncode == 0

    if no_rebuild and image_exists:
        return

    current_hash = get_sandbox_hash()

    if rebuild:
        print(
            f"\033[33mRebuilding sandbox image ({IMAGE_TAG}) as requested...\033[0m"
        )
    elif not image_exists:
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
                f"\033[33mEnvironment configuration changed. Rebuilding sandbox image ({IMAGE_TAG})...\033[0m"
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
        str(DOTFILES_DIR),
    ]

    subprocess.run(build_cmd, check=True)


def get_mount_args():
    args = []
    seen_container_paths = set()

    # Standard Mounts
    for mount in MOUNTS:
        host_path = expand_path(mount.host_path)

        # Only mount if it exists, or if it's the workspace (.)
        if mount.host_path != "." and not host_path.exists():
            continue

        container_path = mount.container_path
        if container_path in seen_container_paths:
            continue
        seen_container_paths.add(container_path)

        ro_flag = ":ro" if mount.ro else ""
        args.extend(["--volume", f"{host_path}:{container_path}{ro_flag}"])

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
        "--no-rebuild",
        action="store_true",
        help="Avoid rebuilding the image if it already exists",
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
        build_image(args.rebuild, args.no_rebuild)
    except subprocess.CalledProcessError:
        sys.exit(1)

    mount_args = get_mount_args()
    env_and_ssh_args = get_env_and_ssh_args(args.with_ssh)

    cmd = (
        [
            "docker",
            "run",
            "-it",
            "--rm",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges",
        ]
        + mount_args
        + env_and_ssh_args
        + ["--workdir", str(WORKSPACE_PATH), IMAGE_TAG]
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
        cmd.append("/usr/bin/fish")

    # Run the container, replacing current process
    os.execvp("docker", cmd)


if __name__ == "__main__":
    main()
