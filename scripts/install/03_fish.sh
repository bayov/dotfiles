#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Fish Shell"
echo "================================================================================"
echo -e "\033[0m"

source "$(dirname "$0")/sudo.sh"

# Set apt-get options for Docker
APT_OPTS=""
if [ "$DOCKER_SANDBOX" = "1" ]; then
    APT_OPTS="--no-install-recommends"
fi

$SUDO apt-add-repository -y ppa:fish-shell/release-4
$SUDO apt-get update
$SUDO apt-get install -y $APT_OPTS fish

# Set fish as default shell for the current user (if not root)
if [ "$(id -u)" -ne 0 ]; then
    CURRENT_USER=$(whoami)
    echo "Setting fish as default shell for $CURRENT_USER..."
    $SUDO chsh -s /usr/bin/fish "$CURRENT_USER"
fi

# Clean up apt cache to reduce image size if in Docker
if [ "$DOCKER_SANDBOX" = "1" ]; then
    $SUDO rm -rf /var/lib/apt/lists/*
fi

# Print version
fish --version

# To update just run again
