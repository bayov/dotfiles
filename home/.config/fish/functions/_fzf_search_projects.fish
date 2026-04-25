function _fzf_search_projects --description "Search projects in defined directories and cd into the selected one."
    set -l direct_paths $fzf_project_paths
    set -l search_paths $fzf_project_search_paths
    if not set -q direct_paths[1]; and not set -q search_paths[1]
        set search_paths ~/projects
    end

    set -f fd_cmd (command -v fdfind || command -v fd || echo "fd")

    # Filter out non-existent directories to avoid fd errors
    set -l valid_directories
    for dir in $direct_paths
        set -l expanded_dir (eval echo -- $dir)
        if test -d "$expanded_dir"
            set -a valid_directories "$expanded_dir"
        end
    end

    set -l valid_search
    for dir in $search_paths
        set -l expanded_dir (eval echo -- $dir)
        if test -d "$expanded_dir"
            set -a valid_search "$expanded_dir"
        end
    end

    if test -z "$valid_directories"; and test -z "$valid_search"
        echo "None of the project or search directories exist." >&2
        return 1
    end

    # Search for directories:
    # 1. Directly in valid_directories (use printf to list them)
    # 2. Children of valid_search (min-depth 1, max-depth 1)
    set -f project_selected (
        begin
            if test -n "$valid_directories"
                printf "%s\n" $valid_directories
            end
            if test -n "$valid_search"
                $fd_cmd --max-depth 1 --min-depth 1 --type d . $valid_search
            end
        end | string replace --regex "^$HOME" "~" | sort -u | fzf --prompt="Project ❯ " --query=(commandline) --ansi --preview-window=70% --preview="$fzf_preview_dir_cmd (string replace --regex '^~' $HOME {})"
    )

    if test $status -eq 0 && test -n "$project_selected"
        commandline --replace -- $project_selected
    end

    commandline --function repaint
end
