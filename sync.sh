#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

USAGE="Usage: $0 [--undeploy] [--dry-run] [--force]"

# Parse arguments
DRY_RUN=""
OPERATION="deploy"
FORCE=""

for arg in "$@"; do
    case "$arg" in
        --undeploy)
            OPERATION="undeploy"
            ;;
        --dry-run)
            DRY_RUN="--dry-run"
            ;;
        --force)
            FORCE="--force"
            ;;
        -h|--help)
            echo "$USAGE"
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg"
            echo "$USAGE"
            exit 1
            ;;
    esac
done

IS_WSL=false
if grep -qi "microsoft" /proc/version 2>/dev/null || grep -qi "wsl" /proc/version 2>/dev/null; then
    IS_WSL=true
fi

LOCAL_CONFIG="dotter/linux.local.toml"
if [ "$IS_WSL" = true ]; then
    echo "Detected WSL environment"
    LOCAL_CONFIG="dotter/wsl.local.toml"
else
    echo "Detected regular Linux environment"
fi

dotter --verbose --global-config dotter/global.toml --local-config "$LOCAL_CONFIG" "$OPERATION" $DRY_RUN $FORCE

if [ "$IS_WSL" = true ]; then
    ./scripts/deploy_windows_shortcuts.sh "$OPERATION" $DRY_RUN
    ./scripts/deploy_vscode_settings.sh "$OPERATION" $DRY_RUN $FORCE
fi
