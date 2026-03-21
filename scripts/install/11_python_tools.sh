#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Python Tools"
echo "================================================================================"
echo -e "\033[0m"

curl -LsSf https://astral.sh/uv/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"
uv tool install dprint-py
