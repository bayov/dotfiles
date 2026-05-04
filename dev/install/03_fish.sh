#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Fish Shell"
echo "================================================================================"
echo -e "\033[0m"

source "$(dirname "$0")/sudo.sh"

if [ "$(uname)" = "Darwin" ]; then
	# Activate brew env so this script works even if PATH wasn't refreshed
	if [ -x /opt/homebrew/bin/brew ]; then
		eval "$(/opt/homebrew/bin/brew shellenv)"
	elif [ -x /usr/local/bin/brew ]; then
		eval "$(/usr/local/bin/brew shellenv)"
	fi

	brew install fish

	FISH_PATH="$(command -v fish)"

	# Register fish in /etc/shells so chsh accepts it. Both /etc/shells edits
	# and chsh itself require sudo; if no TTY is attached (e.g. an automated
	# run), skip with a clear notice instead of failing the whole install.
	if ! grep -qx "$FISH_PATH" /etc/shells; then
		if sudo -n true 2>/dev/null; then
			echo "$FISH_PATH" | $SUDO tee -a /etc/shells >/dev/null
		else
			echo "⚠️  sudo unavailable non-interactively; skipping /etc/shells update."
			echo "    Run manually: echo $FISH_PATH | sudo tee -a /etc/shells"
		fi
	fi

	# Set fish as default shell for the current user
	if [ "$(id -u)" -ne 0 ] && [ "$SHELL" != "$FISH_PATH" ] \
		&& grep -qx "$FISH_PATH" /etc/shells; then
		echo "Setting fish as default shell for $(whoami)..."
		chsh -s "$FISH_PATH" || \
			echo "⚠️  chsh failed; run manually: chsh -s $FISH_PATH"
	fi
else
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
fi

# Print version
fish --version
