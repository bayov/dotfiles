#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Claude Code"
echo "================================================================================"
echo -e "\033[0m"

curl -fsSL https://claude.ai/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"

# Print version
claude --version

# No need to update (claude code updates automatically, so skip this when updating)