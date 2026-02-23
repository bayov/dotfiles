function _fzf_search_jj_status --description "Search the output of jj status. Replace the current token with the selected file paths."
    if not jj root >/dev/null 2>&1
        echo There is no jj repo in (pwd) >&2
        commandline --function repaint
        return
    end

    set -f jj_status_output (jj diff --summary --color=always)
    if test -z "$jj_status_output"
        echo "The working copy has no changes." >&2
        commandline --function repaint
        return
    end

    set -f preview_cmd 'jj diff --color=always -- (string sub --start 3 {})'
    if set --query fzf_diff_highlighter
        set preview_cmd "$preview_cmd | $fzf_diff_highlighter"
    end

    set -f selected_paths (
        jj diff --summary --color=always |
        _fzf_wrapper --ansi \
            --multi \
            --prompt="JJ Status> " \
            --query=(commandline --current-token) \
            --preview=$preview_cmd \
            --preview-window bottom:80% \
            --nth="2.." \
            $fzf_jj_status_opts
    )
    if test $status -eq 0
        set -f cleaned_paths
        for path in $selected_paths
            set --append cleaned_paths (string sub --start 3 $path)
        end

        commandline --current-token --replace -- (string escape -- $cleaned_paths | string join ' ')
    end

    commandline --function repaint
end
