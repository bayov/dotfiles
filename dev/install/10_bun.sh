#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  bun"
echo "================================================================================"
echo -e "\033[0m"

curl -fsSL https://bun.sh/install | bash

# Print version
echo "bun version: $(bun --version)"
