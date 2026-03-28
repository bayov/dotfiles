#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  fzf"
echo "================================================================================"
echo -e "\033[0m"

if [ ! -d "$HOME/.fzf" ]; then
    git clone --depth 1 https://github.com/junegunn/fzf.git "$HOME/.fzf"
else
    (cd "$HOME/.fzf" && git pull)
fi

"$HOME/.fzf/install" --all --no-update-rc --no-bash --no-fish --no-zsh

# Print version
echo "fzf version: $(fzf --version)"
