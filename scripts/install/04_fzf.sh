#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Installing fzf"
echo "================================================================================"
echo -e "\033[0m"

if [ ! -d "$HOME/.fzf" ]; then
    git clone --depth 1 https://github.com/junegunn/fzf.git "$HOME/.fzf"
    "$HOME/.fzf/install" --all
fi

# Print version
echo "fzf version: $(fzf --version)"

# To update, run `cd $HOME/.fzf && git pull && ./install``
