#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

USAGE="Usage: $0 (deploy|undeploy) [--dry-run] [--force]"

# Parse arguments
DRY_RUN=""
OPERATION=""
FORCE=""

for arg in "$@"; do
    case "$arg" in
        deploy|undeploy)
            OPERATION="$arg"
            ;;
        --dry-run)
            DRY_RUN="--dry-run"
            ;;
        --force)
            FORCE="--force"
            ;;
        *)
            echo "$USAGE"
            exit 1
            ;;
    esac
done

if [ -z "$OPERATION" ]; then
    echo "$USAGE"
    exit 1
fi

dotter --global-config dotter/global.toml --local-config dotter/linux.local.toml "$OPERATION" $DRY_RUN $FORCE
