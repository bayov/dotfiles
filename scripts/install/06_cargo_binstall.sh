#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Cargo Binstall"
echo "================================================================================"
echo -e "\033[0m"

curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
