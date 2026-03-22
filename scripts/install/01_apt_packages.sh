#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  APT Packages"
echo "================================================================================"
echo -e "\033[0m"

source "$(dirname "$0")/sudo.sh"

# Set apt-get options for Docker
APT_OPTS=""
if [ "$DOCKER_SANDBOX" = "1" ]; then
    APT_OPTS="--no-install-recommends"
fi

$SUDO apt-get update

# install general pre-requisites
$SUDO apt-get install -y $APT_OPTS software-properties-common
$SUDO apt-get install -y $APT_OPTS build-essential
$SUDO apt-get install -y $APT_OPTS pkg-config
$SUDO apt-get install -y $APT_OPTS libssl-dev
$SUDO apt-get install -y $APT_OPTS gnupg
$SUDO apt-get install -y $APT_OPTS unzip
$SUDO apt-get install -y $APT_OPTS git
$SUDO apt-get install -y $APT_OPTS curl
$SUDO apt-get install -y $APT_OPTS ca-certificates
$SUDO apt-get install -y $APT_OPTS python3
$SUDO apt-get install -y $APT_OPTS python3-venv
$SUDO apt-get install -y $APT_OPTS clang  # For compiling Rust with wild-linker
$SUDO apt-get install -y $APT_OPTS fd-find
$SUDO apt-get install -y $APT_OPTS ripgrep
$SUDO apt-get install -y $APT_OPTS btop
$SUDO apt-get install -y $APT_OPTS bat

# Clean up apt cache to reduce image size if in Docker
if [ "$DOCKER_SANDBOX" = "1" ]; then
    $SUDO rm -rf /var/lib/apt/lists/*
fi
