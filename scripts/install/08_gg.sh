#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing gg"
echo "================================================================================"
echo -e "\033[0m"

export PATH="$HOME/.cargo/bin:$PATH"

sudo apt-get update
sudo apt-get install -y libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev
cargo install gg-cli
