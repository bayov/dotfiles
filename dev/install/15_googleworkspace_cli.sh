#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Google Workspace CLI"
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

	brew install googleworkspace-cli
else
	# cargo was installed by 06_rust.sh into ~/.cargo/bin but PATH may not yet include it
	export PATH="$HOME/.cargo/bin:$PATH"
	cargo install --git https://github.com/googleworkspace/cli --locked
fi

# Print version
gws --version
