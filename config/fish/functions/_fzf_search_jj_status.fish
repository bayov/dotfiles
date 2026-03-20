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

    set -f preview_cmd 'jj diff -w --color=always -- (_fzf_parse_jj_diff_path {})'
    if set --query fzf_diff_highlighter
        set preview_cmd "$preview_cmd | $fzf_diff_highlighter"
    end

    set -fx fzf_jj_status_command jj diff --summary --color=always

    set -f fzf_directory_header "[Alt+E] Edit file · [Ctrl+R] Restore file"
    set -f selected_paths (
        eval "$fzf_jj_status_command" |
        fzf \
            --header="$fzf_directory_header" \
            --ansi \
            --multi \
            --prompt="JJ Status ❯ " \
            --query=(commandline --current-token) \
            --preview=$preview_cmd \
            --preview-window bottom:80% \
            --nth="2.." \
            --height=100% \
            --bind="alt-e:execute($EDITOR (_fzf_parse_jj_diff_path {}))+abort" \
            --bind="ctrl-r:execute(jj restore (_fzf_parse_jj_diff_path {}))+reload(fish -c '$fzf_jj_status_command')" \
            $fzf_jj_status_opts
    )
    if test $status -eq 0
        set -f cleaned_paths
        for path in $selected_paths
            set --append cleaned_paths (_fzf_parse_jj_diff_path $path)
        end

        commandline --current-token --replace -- (string escape -- $cleaned_paths | string join ' ')
    end

    commandline --function repaint
end
