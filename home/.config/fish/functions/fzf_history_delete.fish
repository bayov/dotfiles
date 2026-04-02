function fzf_history_delete
    read -z -l raw_input

    # Assumes fzf history entries are formatted as `{timestamp} │ {cmd}`.
    # See `fzf_history_time_format` var for the exact format.
    set -l parts (string split -m 1 ' │ ' -- $raw_input)

    # Clean timestamp (strip hidden ANSI colors and whitespace)
    set -l timestamp (string replace -r -a '\e\[[0-9;]*m' '' -- $parts[1] | string trim)

    # Collect command (`string collect` to ensure newlines remain)
    set -l cmd (string collect -- $parts[2])

    # Store backup of deleted history entries, just in case.
    printf '⎯⎯⎯⎯⎯[ %s ]⎯⎯⎯⎯⎯\n%s\n\n' "$timestamp" "$cmd" >>$fzf_history_deleted_backup_file

    # Delete history entry.
    history delete --exact --case-sensitive -- "$cmd"
end
