#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Rust (nightly toolchain)"
echo "================================================================================"
echo -e "\033[0m"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --default-toolchain nightly
export PATH="$HOME/.cargo/bin:$PATH"
rustup toolchain install nightly --profile default
rustup default nightly

# Print versions
rustup show
rustc --version
cargo --version

# cranelift for faster dev compile-times
rustup component add rustc-codegen-cranelift-preview --toolchain nightly

# Update using `rustup update`
