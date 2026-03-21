#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Cargo Tools"
echo "================================================================================"
echo -e "\033[0m"

export PATH="$HOME/.cargo/bin:$PATH"

cargo binstall -y --secure dotter
cargo binstall -y --secure jj-cli
cargo binstall -y --secure jj-starship
cargo binstall -y --secure zellij
cargo binstall -y --secure eza
cargo binstall -y --secure bat
cargo binstall -y --secure git-delta
cargo binstall -y --secure cargo-update
cargo binstall -y --secure fresh-editor
cargo binstall -y --secure sccache
cargo binstall -y --secure cargo-nextest
