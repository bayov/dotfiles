#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing Node.js"
echo "================================================================================"
echo -e "\033[0m"

curl -o- https://fnm.vercel.app/install | bash -s -- --install-dir "$HOME/.fnm" --skip-shell
export PATH="$HOME/.fnm:$PATH"
eval "`fnm env`"
fnm install 24
fnm default 24

curl -fsSL https://get.pnpm.io/install.sh | sh -
