#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Cargo Binstall & Cargo Packages"
echo "================================================================================"
echo -e "\033[0m"

# Install cargo-binstall if not already present
export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v cargo-binstall >/dev/null 2>&1; then
	curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
fi

cargo binstall -y --secure cargo-binstall # updates cargo-binstall
cargo binstall -y --secure jj-cli
cargo binstall -y --secure eza
cargo binstall -y --secure git-delta
cargo binstall -y --secure fresh-editor
cargo binstall -y --secure stylua

# For speeding up compilation
cargo binstall -y --secure sccache
cargo binstall -y --secure wild-linker

# For better test runner
cargo binstall -y --secure cargo-nextest

# The following are not needed in docker sandbox
if [ "$DOCKER_SANDBOX" != "1" ]; then
	cargo binstall -y --secure dotter
	cargo binstall -y --secure jj-starship
	cargo binstall -y --secure zellij
	cargo binstall -y --secure cargo-update
fi
