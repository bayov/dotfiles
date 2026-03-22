#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Starship"
echo "================================================================================"
echo -e "\033[0m"

curl -sS https://starship.rs/install.sh | sudo sh -s -- -y

# Print version
starship --version

# To update just run again
