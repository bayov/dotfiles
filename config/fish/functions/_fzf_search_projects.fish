function _fzf_search_projects --description "Search projects in defined directories and cd into the selected one."
    set -l direct_paths $fzf_project_paths
    set -l search_paths $fzf_project_search_paths
    if not set -q direct_paths[1]; and not set -q search_paths[1]
        set search_paths ~/projects
    end

    set -f fd_cmd (command -v fdfind || command -v fd || echo "fd")

    # Filter out non-existent directories to avoid fd errors
    set -l valid_direct
    for dir in $direct_paths
        set -l expanded_dir (eval echo -- $dir)
        if test -d "$expanded_dir"
            set -a valid_direct "$expanded_dir"
        end
    end

    set -l valid_search
    for dir in $search_paths
        set -l expanded_dir (eval echo -- $dir)
        if test -d "$expanded_dir"
            set -a valid_search "$expanded_dir"
        end
    end

    if test -z "$valid_direct"; and test -z "$valid_search"
        echo "None of the project or search directories exist." >&2
        return 1
    end

    # Search for directories:
    # 1. Directly in valid_direct (use printf to list them)
    # 2. Children of valid_search (min-depth 1, max-depth 1)
    set -f project_selected (
        begin
            if test -n "$valid_direct"
                printf "%s\n" $valid_direct
            end
            if test -n "$valid_search"
                $fd_cmd --max-depth 1 --min-depth 1 --type d . $valid_search
            end
        end | string replace --regex "^$HOME" "~" | sort -u | _fzf_wrapper --prompt="Project> " --ansi --preview-window=70% --preview="ls_with_default_args --color=always (string replace --regex '^~' $HOME {})")

    if test $status -eq 0 && test -n "$project_selected"
        # Expand ~ back to $HOME before cd
        set -f project_path (string replace --regex '^~' "$HOME" "$project_selected")
        cd "$project_path"
    end

    commandline --function repaint
end
