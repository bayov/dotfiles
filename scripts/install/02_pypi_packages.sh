#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing PyPi packages"
echo "================================================================================"
echo -e "\033[0m"

# Install `uv` (to update run `uv self update`)
curl -LsSf https://astral.sh/uv/install.sh | sh
export PATH="$HOME/.local/bin:$PATH"

# Print version
uv --version

# Install package using `uv` (to update, replace `install` with `update`).
$HOME/.local/bin/uv tool install dprint-py
