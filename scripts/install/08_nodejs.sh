#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Node.js (using fnm) & PNPM"
echo "================================================================================"
echo -e "\033[0m"

# Download and install fnm
INSTALL_DIR="$HOME/.local/share/fnm"
curl -o- https://fnm.vercel.app/install | bash -s -- --install-dir "$INSTALL_DIR" --skip-shell

# Activate fnm
fnm env

# Download and install latest Node.js with LTS
fnm install --lts
node -v  # prints version

# Download and install pnpm (faster package manager)
corepack prepare pnpm@latest --activate
pnpm -v  # prints version

# To update just run again
