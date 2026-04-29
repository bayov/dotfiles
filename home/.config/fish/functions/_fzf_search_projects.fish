function _fzf_search_projects --description "Search projects in defined directories and cd into the selected one."
    set -l roots $fzf_project_roots
    if not set -q roots[1]
        set roots ~/projects
    end

    set -l valid_roots
    for dir in $roots
        set -l expanded (eval echo -- $dir)
        if test -d "$expanded"
            set -a valid_roots "$expanded"
        end
    end

    if not set -q valid_roots[1]
        echo "None of the project root directories exist." >&2
        return 1
    end

    set -f project_selected (
        _fzf_find_projects $valid_roots | string replace --regex "^$HOME" "~" | sort -u | fzf --prompt="Project ❯ " --query=(commandline) --ansi --preview-window=70% --preview="$fzf_preview_dir_cmd (string replace --regex '^~' $HOME {})"
    )

    if test $status -eq 0 && test -n "$project_selected"
        commandline --replace -- $project_selected
    end

    commandline --function repaint
end
