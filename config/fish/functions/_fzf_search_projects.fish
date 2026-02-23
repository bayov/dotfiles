function _fzf_search_projects --description "Search projects in defined directories and cd into the selected one."
    set -l projects_dirs $fzf_project_paths
    if not set -q projects_dirs[1]
        set projects_dirs ~/projects
    end

    # Filter out non-existent directories to avoid fd errors
    set -l search_paths
    for dir in $projects_dirs
        set -l expanded_dir (eval echo -- $dir)
        if test -d "$expanded_dir"
            set -a search_paths "$expanded_dir"
        end
    end

    if test -z "$search_paths"
        echo "None of the project directories exist: $projects_dirs" >&2
        return 1
    end

    set -f fd_cmd (command -v fdfind || command -v fd || echo "fd")
    # Search for directories one level deep in all search_paths.
    # Replace $HOME with ~ for display purposes in fzf.
    set -f project_selected ($fd_cmd --max-depth 1 --min-depth 1 --type d . $search_paths | string replace --regex "^$HOME" "~" | _fzf_wrapper --prompt="Project> " --preview="_fzf_preview_file (string replace --regex '^~' $HOME {})")

    if test $status -eq 0 && test -n "$project_selected"
        # Expand ~ back to $HOME before cd
        set -f project_path (string replace --regex '^~' "$HOME" "$project_selected")
        cd "$project_path"
    end

    commandline --function repaint
end
