#!/bin/bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Execute all install scripts in order
for script in "$DIR"/scripts/install/*.sh; do
    if [ -x "$script" ]; then
        filename=$(basename "$script")
        echo "Running $filename..."
        "$script"
    fi
done


echo -e "\033[32m"
echo "================================================================================"
echo "  All prereqs installed successfully!"
echo "================================================================================"
echo -e "\033[0m"
