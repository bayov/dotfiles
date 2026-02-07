#!/usr/bin/env bash
set -euo pipefail

# Pensieve — Install as git submodule
# Run from the root of your project repository.

# Defaults
SUBMODULE_DIR=".agents/pensieve"
SKILLS_DIR=".agents/skills"
REPO="git@github.com:bayov/pensieve.git"

# Parse flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    --submodule-dir) SUBMODULE_DIR="$2"; shift 2 ;;
    --skills-dir)    SKILLS_DIR="$2";    shift 2 ;;
    --repo)          REPO="$2";          shift 2 ;;
    *)
      echo "Usage: install.sh [--submodule-dir .agents/pensieve] [--skills-dir .agents/skills] [--repo git@github.com:bayov/pensieve.git]" >&2
      exit 1
      ;;
  esac
done

# 1. Add submodule
if [ -d "$SUBMODULE_DIR" ]; then
  echo "Submodule already exists at $SUBMODULE_DIR, skipping."
else
  echo "Adding pensieve submodule..."
  git submodule add "$REPO" "$SUBMODULE_DIR"
fi

# 2. Symlink each skill into {dir}/skills/{skill-name}
mkdir -p "$SKILLS_DIR"

for skill_dir in "$SUBMODULE_DIR"/src/skills/*/; do
  skill_name="$(basename "$skill_dir")"
  link="$SKILLS_DIR/$skill_name"
  # Use relative path so the symlink is portable
  target="$(realpath --relative-to="$SKILLS_DIR" "$skill_dir")"

  if [ -L "$link" ]; then
    echo "Symlink already exists: $link"
  else
    ln -s "$target" "$link"
    echo "Linked $link -> $target"
  fi
done

echo ""
echo "Done! Next steps:"
echo "  1. Add an AGENTS.md (or CLAUDE.md) to your project root."
echo "     See the example in the pensieve README for the recommended template."
echo "  2. Commit the changes."
