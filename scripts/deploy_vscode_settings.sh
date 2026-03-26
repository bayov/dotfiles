#!/bin/bash
set -e

USAGE="Usage: $0 (deploy|undeploy) [--dry-run]"

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
            DRY_RUN="yes"
            ;;
        --force)
            FORCE="yes"
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

# Define paths
# Target paths in Windows (where the symlinks will be)
WIN_SETTINGS_PATH="\$env:APPDATA\Code\User\settings.json"
WIN_KEYBINDINGS_PATH="\$env:APPDATA\Code\User\keybindings.json"

# Source paths in WSL (where the actual files are)
# We construct the network path accessible from Windows
DISTRO_NAME="${WSL_DISTRO_NAME:-Ubuntu}"
# Get absolute path to dotfiles root (assuming we run from root)
REPO_ROOT=$(pwd)
# Convert to Windows style backslashes for the path part
REPO_ROOT_WIN=$(echo "$REPO_ROOT" | sed 's|/|\\|g')

WSL_SETTINGS_PATH="\\\\wsl.localhost\\$DISTRO_NAME$REPO_ROOT_WIN\\config\\Code\\User\\settings.json"
WSL_KEYBINDINGS_PATH="\\\\wsl.localhost\\$DISTRO_NAME$REPO_ROOT_WIN\\config\\Code\\User\\keybindings.json"

# Function to execute PowerShell commands
# usage: run_powershell_elevated <command> <log_line>
run_powershell_elevated() {
    local cmd="$1"
    local log_line="$2"

    echo -e "$log_line${DRY_RUN:+ (dry run)}"
    
    if [ -n "$DRY_RUN" ]; then
        return 0
    fi

    # Encode command to UTF-16LE Base64 for safe passing to PowerShell
    local encoded_cmd=$(printf "%s" "$cmd" | iconv -f UTF-8 -t UTF-16LE | base64 -w 0)
    
    # Execute with RunAs (Elevated)
    powershell.exe -NoProfile -NonInteractive -Command \
        "Start-Process powershell -Verb RunAs -Wait -ArgumentList '-NoProfile -ExecutionPolicy Bypass -EncodedCommand $encoded_cmd'"
}

# Check if a link exists and points to the correct target
# usage: check_link <win_path> <wsl_target>
# returns: 0 if correct link exists, 1 otherwise
check_link() {
    local win_path="$1"
    local wsl_target="$2"

    # We use a PowerShell check. 
    # We normalize both paths to handle UNC prefixes and array-to-string conversion.
    local check_cmd="
        \$p = \"$win_path\"
        \$t = \"$wsl_target\"
        if (-not (Test-Path \$p)) { exit 1 }
        \$i = Get-Item \$p
        if (\$i.LinkType -ne 'SymbolicLink') { exit 1 }
        
        # Normalize target: remove leading \\ or UNC\ and convert backslashes to a consistent format
        \$actual = (\$i.Target -join '').Replace('UNC\', '').Replace('\\\\', '').Trim('\\')
        \$expected = \$t.Replace('UNC\', '').Replace('\\\\', '').Trim('\\')
        
        if (\$actual -eq \$expected) { exit 0 }
        exit 1
    "
    
    # Run check non-interactively, suppress output
    powershell.exe -NoProfile -NonInteractive -Command "$check_cmd" > /dev/null 2>&1
}

deploy_link() {
    local rel_path="$1"
    local win_path="$2"
    local wsl_target="$3"

    # Define colors for log line
    local cyan="\033[0;36m"
    local magenta="\033[1;35m"
    local green="\033[0;32m"
    local nc="\033[0m"

    # Escape backslashes for echo -e
    local rel_path_escaped="${rel_path//\\/\\\\}"
    local wsl_target_escaped="${wsl_target//\\/\\\\}"
    local log_line="${cyan}PS symlink${nc} \"${magenta}${rel_path_escaped}${nc}\"\n  ↳ \"${magenta}${wsl_target_escaped}${nc}\""

    if [ -z "$FORCE" ]; then
        if check_link "$win_path" "$wsl_target"; then
             echo -e "$log_line: ${green}Already linked (skipping)${nc}"
             return 0
        fi
    fi

    local cmd="New-Item -ItemType SymbolicLink -Path \"$win_path\" -Target \"$wsl_target\" -Force"
    run_powershell_elevated "$cmd" "$log_line"
}

undeploy_link() {
    local win_path="$1"

    # Define colors for log line
    local cyan="\033[0;36m"
    local magenta="\033[1;35m"
    local green="\033[0;32m"
    local nc="\033[0m"

    # Escape backslashes for echo -e
    local win_path_escaped="${win_path//\\/\\\\}"
    local log_line="${cyan}PS rm${nc} \"${magenta}${win_path_escaped}${nc}\""

    # Check if exists
    local check_cmd="if (Test-Path \"$win_path\") { exit 0 } else { exit 1 }"
    if ! powershell.exe -NoProfile -NonInteractive -Command "$check_cmd" > /dev/null 2>&1; then
        echo -e "$log_line: ${green}Not present (skipping)${nc}"
        return 0
    fi

    local cmd="Remove-Item \"$win_path\" -Force"
    run_powershell_elevated "$cmd" "$log_line"
}

# Relative paths for logging
REL_SETTINGS="%appdata%\\config\\Code\\User\\settings.json"
REL_KEYBINDINGS="%appdata%\\config\\Code\\User\\keybindings.json"

if [ "$OPERATION" = "deploy" ]; then
    deploy_link "$REL_SETTINGS" "$WIN_SETTINGS_PATH" "$WSL_SETTINGS_PATH"
    deploy_link "$REL_KEYBINDINGS" "$WIN_KEYBINDINGS_PATH" "$WSL_KEYBINDINGS_PATH"

elif [ "$OPERATION" = "undeploy" ]; then
    undeploy_link "$WIN_SETTINGS_PATH"
    undeploy_link "$WIN_KEYBINDINGS_PATH"
fi
