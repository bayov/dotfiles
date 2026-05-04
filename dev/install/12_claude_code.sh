#!/bin/bash
set -e

echo -e "\033[34m"
echo "================================================================================"
echo "  Claude Code"
echo "================================================================================"
echo -e "\033[0m"

# No need to manually update (claude code updates automatically).
if ! command -v claude >/dev/null 2>&1; then
	curl -fsSL https://claude.ai/install.sh | bash
fi
export PATH="$HOME/.local/bin:$PATH"

# Activate fnm so npx is available
export PATH="$HOME/.local/share/fnm:$PATH"
eval "$(fnm env --shell bash)"

# `playwright install chrome` writes to /Applications on macOS and needs sudo;
# the user-cache `chromium` build avoids that and works the same for Claude Code.
# Allow failure here: corporate TLS proxies often break the CDN download, but
# the rest of the toolchain is still usable; the user can retry later.
if [ "$(uname)" = "Darwin" ]; then
	npx --yes playwright install chromium || \
		echo "⚠️  playwright install chromium failed; rerun manually after fixing TLS/proxy."
else
	npx --yes playwright install chrome
fi

# Print version
claude --version
