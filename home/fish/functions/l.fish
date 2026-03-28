function l --description 'List directory contents or cat file based on input type'
    # Extract only positional arguments (those not starting with -)
    set -l positions (string match -v -- "-*" $argv)

    # Error if more than one positional argument is detected
    if test (count $positions) -gt 1
        echo "Error: Multiple positional arguments"
        return 1
    end

    if test (count $positions) -eq 0
        # Default behavior: list current directory
        ls_with_default_args . $argv
    else if test -d "$positions[1]"
        # If the argument is a directory, use ll
        ls_with_default_args $argv
    else
        # If the argument is a file (or anything else), use cat
        cat $argv
    end
end
