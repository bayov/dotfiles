#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Platform Specific Packages"
echo "================================================================================"
echo -e "\033[0m"

source "$(dirname "$0")/sudo.sh"

if [ "$(uname)" = "Darwin" ]; then
	echo "Detected macOS. Installing packages via Homebrew..."

	# Ensure Xcode Command Line Tools are present (provides clang, git, etc.)
	if ! xcode-select -p >/dev/null 2>&1; then
		echo "Installing Xcode Command Line Tools (a GUI prompt may appear)..."
		xcode-select --install || true
		until xcode-select -p >/dev/null 2>&1; do
			sleep 5
		done
	fi

	# Ensure Homebrew is installed and on PATH for this shell
	if ! command -v brew >/dev/null 2>&1; then
		echo "Installing Homebrew..."
		NONINTERACTIVE=1 /bin/bash -c \
			"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
	fi
	if [ -x /opt/homebrew/bin/brew ]; then
		eval "$(/opt/homebrew/bin/brew shellenv)"
	elif [ -x /usr/local/bin/brew ]; then
		eval "$(/usr/local/bin/brew shellenv)"
	fi

	brew update

	# macOS counterparts of the Debian package list. Most equivalents are
	# directly named; build-essential/libssl-dev/clang are covered by Xcode
	# CLT + openssl@3, and snap-based glow becomes a brew formula.
	brew install \
		pkg-config \
		openssl@3 \
		gnupg \
		unzip \
		git \
		curl \
		python3 \
		fd \
		ripgrep \
		btop \
		bat \
		shfmt \
		glow \
		coreutils
elif [ -f /etc/debian_version ] || [ -f /etc/lsb-release ]; then
	echo "Detected Debian-based system. Installing packages via apt..."

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
	$SUDO apt-get install -y $APT_OPTS clang # For compiling Rust with wild-linker
	$SUDO apt-get install -y $APT_OPTS fd-find
	$SUDO apt-get install -y $APT_OPTS ripgrep
	$SUDO apt-get install -y $APT_OPTS btop
	$SUDO apt-get install -y $APT_OPTS bat
	$SUDO apt-get install -y $APT_OPTS shfmt

	# Install glow via snap if available
	if command -v snap >/dev/null 2>&1; then
		$SUDO snap install glow
	fi

	# Clean up apt cache to reduce image size if in Docker
	if [ "$DOCKER_SANDBOX" = "1" ]; then
		$SUDO rm -rf /var/lib/apt/lists/*
	fi
else
	echo "Skipping platform-specific installation (unknown system)."
fi
