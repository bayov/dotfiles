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

    set -f preview_cmd 'set -l path (_fzf_parse_jj_diff_path {}); jj diff -w --color=always -- "file:\"$path\""'
    if set --query fzf_diff_highlighter
        set preview_cmd "$preview_cmd | $fzf_diff_highlighter"
    end

    set -fx fzf_jj_status_command jj diff --summary --color=always

    set -f fzf_directory_header "[Alt+E] Edit file · [Ctrl+R] Restore file"
    set -f fzf_output (
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
            --expect=alt-e \
            --bind="ctrl-r:execute-silent(printf '%s\n' {+} | fish -c 'while read -l line; set -l path (_fzf_parse_jj_diff_path \$line); jj restore \"file:\\\"\$path\\\"\"; end')+reload(fish -c '$fzf_jj_status_command')" \
            --bind="ctrl-y:execute-silent(printf '%s\n' {+} | fish -c 'while read -l line; _fzf_parse_jj_diff_path \$line; end | string collect | clip')" \
            --bind="ctrl-alt-c:execute-silent(printf '%s\n' {+} | fish -c 'while read -l line; _fzf_parse_jj_diff_path \$line; end | string collect | clip')" \
            $fzf_jj_status_opts
    )
    set -f fzf_status $status
    if test $fzf_status -eq 0
        set -f key $fzf_output[1]
        set -f cleaned_paths
        for path in $fzf_output[2..]
            set --append cleaned_paths (_fzf_parse_jj_diff_path $path)
        end
        if test "$key" = alt-e
            $EDITOR $cleaned_paths
        else
            commandline --current-token --replace -- (string escape -- $cleaned_paths | string join ' ')
        end
    end

    commandline --function repaint
end
