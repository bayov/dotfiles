#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Claude Code"
echo "================================================================================"
echo -e "\033[0m"

# No need to manually update (claude code updates automatically).
if ! command -v claude >/dev/null 2>&1; then
	curl -fsSL https://claude.ai/install.sh | bash
fi
export PATH="$HOME/.local/bin:$PATH"

# Activate fnm so npx is available
export PATH="$HOME/.local/share/fnm:$PATH"
eval "$(fnm env --shell bash)"

npx playwright install chrome

# Print version
claude --version
