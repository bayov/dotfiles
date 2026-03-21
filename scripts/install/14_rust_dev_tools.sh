#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Rust Dev Tools"
echo "================================================================================"
echo -e "\033[0m"

export PATH="$HOME/.cargo/bin:$PATH"

rustup component add rustc-codegen-cranelift-preview --toolchain nightly

sudo apt-get update
sudo apt-get install -y clang
cargo install --locked wild-linker
