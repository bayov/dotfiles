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

# binstall queries the GitHub API for every package; unauthenticated requests
# get rate-limited fast. If `gh` is logged in, hand its token to binstall so we
# stay under the higher authenticated limit and avoid slow source-build fallbacks.
if [ -z "${GITHUB_TOKEN:-}" ] && command -v gh >/dev/null 2>&1; then
	if gh auth status >/dev/null 2>&1; then
		GITHUB_TOKEN="$(gh auth token 2>/dev/null)" && export GITHUB_TOKEN
	fi
fi

cargo binstall -y --secure cargo-binstall # updates cargo-binstall
cargo binstall -y --secure jj-cli
cargo binstall -y --secure eza
cargo binstall -y --secure git-delta
cargo binstall -y --secure fresh-editor
cargo binstall -y --secure stylua

# For speeding up compilation
cargo binstall -y --secure sccache
# wild-linker only targets ELF (Linux); macOS uses Mach-O and Apple's ld.
if [ "$(uname)" != "Darwin" ]; then
	cargo binstall -y --secure wild-linker
fi

# For better test runner
cargo binstall -y --secure cargo-nextest

# The following are not needed in docker sandbox
if [ "$DOCKER_SANDBOX" != "1" ]; then
	cargo binstall -y --secure dotter
	cargo binstall -y --secure jj-starship
	cargo binstall -y --secure zellij
	cargo binstall -y --secure cargo-update
fi
