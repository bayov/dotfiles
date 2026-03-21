#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing AI Tools"
echo "================================================================================"
echo -e "\033[0m"

export PATH="$HOME/.fnm:$PATH"
eval "`fnm env`"
export PNPM_HOME="$HOME/.local/share/pnpm"
export PATH="$PNPM_HOME:$PATH"

pnpm add -g @anthropic-ai/claude-code @google/gemini-cli opencode-ai @fission-ai/openspec@latest
pnpm approve-builds --global --all || true
