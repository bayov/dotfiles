# Helper to run commands with sudo if needed
if [ "$(id -u)" -eq 0 ]; then
	SUDO=""
elif command -v sudo >/dev/null 2>&1; then
	SUDO="sudo"
else
	echo "Error: script is not running as root and sudo is not installed."
	exit 1
fi
