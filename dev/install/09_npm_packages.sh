#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  NPM Packages"
echo "================================================================================"
echo -e "\033[0m"

# Prepare environment
export PATH="$HOME/.local/share/fnm:$PATH"
eval "$(fnm env --shell bash)"
fnm use default
corepack enable pnpm

export PNPM_HOME="$HOME/.local/share/pnpm"
export PATH="$PNPM_HOME:$PATH"

pnpm add -g npx
pnpm add -g @google/gemini-cli@preview
pnpm add -g opencode-ai@latest
pnpm add -g @fission-ai/openspec@latest
pnpm add -g @mariozechner/pi-coding-agent

pnpm approve-builds --global --all || true

# To update just run again
