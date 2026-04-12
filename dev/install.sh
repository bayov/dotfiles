#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Suppress interactive prompts across tools (npm, playwright, etc.)
export CI=true

# Execute all install scripts in order
for script in "$DIR"/install/*.sh; do
	if [ -x "$script" ]; then
		filename=$(basename "$script")
		"$script"
	fi
done

echo -e "\033[32m"
echo "================================================================================"
echo "  All packages installed successfully!"
echo "================================================================================"
echo -e "\033[0m"
