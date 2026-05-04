#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Starship"
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
	brew install starship
else
	curl -sS https://starship.rs/install.sh | $SUDO sh -s -- -y
fi

# Print version
starship --version
