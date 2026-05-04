#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  JetBrains Mono Font"
echo "================================================================================"
echo -e "\033[0m"

source "$(dirname "$0")/sudo.sh"

if [ "$(uname)" = "Darwin" ]; then
	if [ -x /opt/homebrew/bin/brew ]; then
		eval "$(/opt/homebrew/bin/brew shellenv)"
	elif [ -x /usr/local/bin/brew ]; then
		eval "$(/usr/local/bin/brew shellenv)"
	fi
	brew install --cask font-jetbrains-mono
else
	FONT_DIR="$HOME/.local/share/fonts/JetBrainsMono"
	if [ ! -d "$FONT_DIR" ] || [ -z "$(ls -A "$FONT_DIR" 2>/dev/null)" ]; then
		mkdir -p "$FONT_DIR"
		TMP_ZIP="$(mktemp -t jetbrains-mono.XXXXXX.zip)"
		curl -fsSL -o "$TMP_ZIP" \
			https://download.jetbrains.com/fonts/JetBrainsMono-2.304.zip
		unzip -o -q "$TMP_ZIP" -d "$TMP_ZIP.d"
		find "$TMP_ZIP.d/fonts/ttf" -name '*.ttf' -exec cp {} "$FONT_DIR/" \;
		rm -rf "$TMP_ZIP" "$TMP_ZIP.d"
		if command -v fc-cache >/dev/null 2>&1; then
			fc-cache -f "$FONT_DIR"
		fi
	fi
fi
