#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Fish Shell"
echo "================================================================================"
echo -e "\033[0m"

sudo apt-add-repository -y ppa:fish-shell/release-4
sudo apt-get update
sudo apt-get install -y fish

# Print version
fish --version

# To update just run again
