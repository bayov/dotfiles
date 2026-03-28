#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Starship"
echo "================================================================================"
echo -e "\033[0m"

source "$(dirname "$0")/sudo.sh"

curl -sS https://starship.rs/install.sh | $SUDO sh -s -- -y

# Print version
starship --version
