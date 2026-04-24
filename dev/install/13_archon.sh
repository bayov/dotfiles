#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Archon"
echo "================================================================================"
echo -e "\033[0m"

if [ ! -d "$HOME/archon" ]; then
	git clone --depth 1 https://github.com/coleam00/Archon.git "$HOME/archon"
else
	(cd "$HOME/archon" && git pull)
fi
