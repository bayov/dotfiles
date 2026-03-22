#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Node.js (using fnm) & PNPM"
echo "================================================================================"
echo -e "\033[0m"

# Download and install fnm
INSTALL_DIR="$HOME/.local/share/fnm"
curl -o- https://fnm.vercel.app/install | bash -s -- --install-dir "$INSTALL_DIR" --skip-shell

# Activate fnm
export PATH="$INSTALL_DIR:$PATH"
eval "$(fnm env)"

# Download and install latest Node.js with LTS
fnm install --lts
fnm use lts-latest
fnm default lts-latest

# Download and install pnpm (faster package manager)
corepack prepare pnpm@latest --activate

# Print versions
echo "fnm version: $(fnm --version)"
echo "node version: $(node --version)"
echo "pnpm version: $(pnpm --version)"
