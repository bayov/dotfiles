#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

USAGE="Usage: $0 (deploy|undeploy) [--dry-run] [--force] [--google]"

# Parse arguments
DRY_RUN=""
OPERATION=""
FORCE=""
GOOGLE=""

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
        --google)
            GOOGLE="--google"
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

dotter --verbose --global-config dotter/global.toml --local-config dotter/linux.local.toml "$OPERATION" $DRY_RUN $FORCE

if [ -n "$GOOGLE" ]; then
    starship_config="$HOME/.config/starship.toml"
    default_starship_right_format="right_format = \"\$all\""
    google_starship_right_format="right_format = \"\${custom.google3_pwd}\""

    echo "--google mode, replacing $starship_config right_format"
    sed -i "s/^${default_starship_right_format}$/${google_starship_right_format}/" "$starship_config"
fi
