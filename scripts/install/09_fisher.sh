#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Fisher"
echo "================================================================================"
echo -e "\033[0m"

fish -c "curl -sL https://raw.githubusercontent.com/jorgebucaran/fisher/main/functions/fisher.fish | source && fisher install jorgebucaran/fisher"
