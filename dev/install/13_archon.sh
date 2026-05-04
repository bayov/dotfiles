#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Archon"
echo "================================================================================"
echo -e "\033[0m"

# bun was installed by 10_bun.sh into ~/.bun/bin but PATH may not yet include it
export PATH="$HOME/.bun/bin:$PATH"

ARCHON_DIR="${ARCHON_DIR:-$HOME/.archon/archon}"

if [ ! -d "$ARCHON_DIR" ]; then
	git clone --depth 1 https://github.com/coleam00/Archon.git "$ARCHON_DIR"
else
	(cd "$ARCHON_DIR" && git pull)
fi

(cd "$ARCHON_DIR" && bun install)
(cd "$ARCHON_DIR/packages/cli" && bun link)
