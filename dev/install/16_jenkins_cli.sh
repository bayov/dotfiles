#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Jenkins CLI (jcli)"
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

	brew tap jenkins-zh/jcli
	brew install jcli
else
	case "$(uname -m)" in
	x86_64) ARCH="amd64" ;;
	aarch64 | arm64) ARCH="arm64" ;;
	*)
		echo "Unsupported arch: $(uname -m)" >&2
		exit 1
		;;
	esac

	TMP="$(mktemp -d)"
	curl -fsSL "https://github.com/jenkins-zh/jenkins-cli/releases/latest/download/jcli-linux-${ARCH}.tar.gz" | tar xz -C "$TMP"
	$SUDO mv "$TMP/jcli" /usr/local/bin/
	rm -rf "$TMP"
fi

# Print version
jcli version
