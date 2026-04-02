#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  PyPi packages"
echo "================================================================================"
echo -e "\033[0m"

# Update or install `uv`
export PATH="$HOME/.local/bin:$PATH"
if command -v uv >/dev/null 2>&1; then
	uv self update
else
	curl -LsSf https://astral.sh/uv/install.sh | sh
fi

# Print version
uv --version

# Install package using `uv`.
uv tool install --upgrade dprint-py
uv tool install --upgrade ipython
