#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

USAGE="Usage: $0 (deploy|undeploy) [--dry-run]"

# Path to the AutoHotkey loader shortcut file
# This shortcut is responsible for loading all configured AutoHotkey scripts
AHK_LNK="./config/windows_shortcuts/load_all.lnk"
AHL_LNK_BASENAME=$(basename "$AHK_LNK")

# Check if the AHK .lnk exists
if [ ! -f "$AHK_LNK" ]; then
    CURRENT_DIR=$(pwd)
    echo "Error: Source file '$AHK_LNK' not found in '$CURRENT_DIR'."
    echo "Please ensure the file exists at the specified path or update the SOURCE_FILE variable in the script."
    exit 1
fi

# Parse arguments
DRY_RUN=""
OPERATION=""

for arg in "$@"; do
    case "$arg" in
        deploy|undeploy)
            OPERATION="$arg"
            ;;
        --dry-run)
            DRY_RUN="yes"
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

# Define colors
CYAN='\033[0;36m'
MAGENTA='\033[1;35m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Copies a file with status reporting and error handling.
# 
# Parameters:
#   source_item - The path to the source file to copy
#   dest_item   - The path where the file should be copied to
#
# Returns:
#   0 - Success
#   1 - Missing parameters
#   2 - Destination already exists
#   other - cp command error code
copy_with_status() {
    local source_item="$1"
    local dest_item="$2"

    # Check if source and destination are provided
    if [ -z "$source_item" ] || [ -z "$dest_item" ]; then
        echo -e "${RED}Error:${NC} Source and destination must be provided to copy_with_status function."
        return 1 # Return an error code from the function
    fi

    # Check if the destination already exists
    if [ -e "$dest_item" ]; then
        echo -e "${RED}Error:${NC} Destination '${MAGENTA}${dest_item}${NC}' already exists. Will not overwrite."
        return 2 # Return a specific error code for existing destination
    fi

    # Print the command being executed with colors
    echo -e "${CYAN}cp${NC} \"${MAGENTA}${source_item}${NC}\" \"${MAGENTA}${dest_item}${NC}\"${DRY_RUN:+ (dry run)}"
    
    # Execute the copy command unless in dry-run mode
    if [ -z "$DRY_RUN" ]; then
        cp "$source_item" "$dest_item"
        local ret=$? # Use local for the return code variable inside the function

        # Check the return code
        if [ $ret -ne 0 ]; then
            # Print error message in red, with variables in yellow
            echo -e "${RED}Error:${NC} Failed to copy '${MAGENTA}${source_item}${NC}' to '${MAGENTA}${dest_item}${NC}'."
            # exit 1 # Exiting here would terminate the whole script.
                     # It might be better to return an error code and let the caller decide.
            return $ret # Return the error code from cp
        fi
    fi

    return 0
}

# Removes a file with status reporting and error handling.
#
# Parameters:
#   target_file - The path to the file to remove
#
# Returns:
#   0 - Success
#   1 - Missing target parameter
#   2 - Target file does not exist
#   other - rm command error code
remove_with_status() {
    local target_file="$1"

    # Check if target file is provided
    if [ -z "$target_file" ]; then
        echo -e "${RED}Error:${NC} Target file must be provided to remove_with_status function."
        return 1
    fi

    # Check if the file exists
    if [ ! -e "$target_file" ]; then
        echo -e "${RED}Error:${NC} Target file '${MAGENTA}${target_file}${NC}' does not exist."
        return 2
    fi

    # Print the command being executed with colors
    echo -e "${CYAN}rm${NC} \"${MAGENTA}${target_file}${NC}\"${DRY_RUN:+ (dry run)}"
    
    # Execute the remove command unless in dry-run mode
    if [ -z "$DRY_RUN" ]; then
        rm "$target_file"
        local ret=$?

        # Check the return code
        if [ $ret -ne 0 ]; then
            echo -e "${RED}Error:${NC} Failed to remove '${MAGENTA}${target_file}${NC}'."
            return $ret
        fi
    fi

    return 0
}

# Get host Windows %APPDATA% path
WINDOWS_APPDATA_RAW=$(cmd.exe /c echo %APPDATA% 2>/dev/null)
if [ -z "$WINDOWS_APPDATA_RAW" ]; then
    echo "Error: Failed to retrieve %APPDATA% from Windows."
    echo "Please ensure you are running this script within a WSL environment and that cmd.exe is accessible and functioning correctly."
    exit 1
fi

# Remove carriage return characters (CR) often output by cmd.exe
WINDOWS_APPDATA=$(echo "$WINDOWS_APPDATA_RAW" | tr -d '\r')

# Convert the Windows path to a WSL path
APPDATA_DIR=$(wslpath "$WINDOWS_APPDATA")
if [ -z "$APPDATA_DIR" ]; then
    echo "Error: Failed to convert Windows path '$WINDOWS_APPDATA' to WSL path using wslpath."
    echo "Ensure wslpath is installed and working correctly."
    exit 1
fi

# Windows' startup directory
STARTUP_DIR="$APPDATA_DIR/Microsoft/Windows/Start Menu/Programs/Startup"
if [[ ! -d "$STARTUP_DIR" ]]; then
    echo "Error: Windows startup directory '$STARTUP_DIR' does not exist."
    exit 1
fi

# Perform the requested operation
if [ "$OPERATION" = "deploy" ]; then
    copy_with_status "$AHK_LNK" "$STARTUP_DIR/$AHL_LNK_BASENAME"
elif [ "$OPERATION" = "undeploy" ]; then
    remove_with_status "$STARTUP_DIR/$AHL_LNK_BASENAME"
fi

exit 0
