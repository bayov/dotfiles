#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Gum"
echo "================================================================================"
echo -e "\033[0m"

source "$(dirname "$0")/sudo.sh"

# Set apt-get options for Docker
APT_OPTS=""
if [ "$DOCKER_SANDBOX" = "1" ]; then
    APT_OPTS="--no-install-recommends"
fi

$SUDO mkdir -p /etc/apt/keyrings
curl -fsSL https://repo.charm.sh/apt/gpg.key | $SUDO gpg --dearmor --yes -o /etc/apt/keyrings/charm.gpg || true
echo "deb [signed-by=/etc/apt/keyrings/charm.gpg] https://repo.charm.sh/apt/ * *" | $SUDO tee /etc/apt/sources.list.d/charm.list
$SUDO apt-get update
$SUDO apt-get install -y $APT_OPTS gum

# Clean up apt cache to reduce image size if in Docker
if [ "$DOCKER_SANDBOX" = "1" ]; then
    $SUDO rm -rf /var/lib/apt/lists/*
fi

# Print version
gum --version
