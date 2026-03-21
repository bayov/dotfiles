#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Starship"
echo "================================================================================"
echo -e "\033[0m"

curl -sS https://starship.rs/install.sh | sh -s -- -y

# Do not install in Docker sandbox