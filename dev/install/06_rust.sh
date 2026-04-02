#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Rust (nightly toolchain)"
echo "================================================================================"
echo -e "\033[0m"

export PATH="$HOME/.cargo/bin:$PATH"

if ! command -v rustup >/dev/null 2>&1; then
	echo "Installing Rust..."
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain nightly
	rustup toolchain install nightly --profile default
else
	# Fetch the current status
	STATUS=$(rustup check 2>&1 | grep "nightly" || true)
	if [[ $STATUS == *"Update available"* ]]; then
		echo "New nightly version found. Updating..."
		rustup update nightly
	else
		echo "Nightly is already up to date."
	fi
fi

rustup default nightly

# Update codegen-cranelift
rustup component add rustc-codegen-cranelift-preview --toolchain nightly

# Print versions
rustup show
rustc --version
cargo --version
