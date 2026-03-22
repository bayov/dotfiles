#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Claude Code"
echo "================================================================================"
echo -e "\033[0m"

# Prepare environment
export PATH="$HOME/.local/share/fnm:$PATH"
eval "$(fnm env)"
fnm use default

export PNPM_HOME="$HOME/.local/share/pnpm"
export PATH="$PNPM_HOME:$PATH"

curl -fsSL https://claude.ai/install.sh | bash

# Print version
claude --version

# No need to update (claude code updates automatically, so skip this when updating)