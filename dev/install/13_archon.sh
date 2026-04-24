#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Archon"
echo "================================================================================"
echo -e "\033[0m"

ARCHON_DIR="${ARCHON_DIR:-$HOME/.archon/archon}"

if [ ! -d "$ARCHON_DIR" ]; then
	git clone --depth 1 https://github.com/coleam00/Archon.git "$ARCHON_DIR"
else
	(cd "$ARCHON_DIR" && git pull)
fi

(cd "$ARCHON_DIR" && bun install)
(cd "$ARCHON_DIR/packages/cli" && bun link)
