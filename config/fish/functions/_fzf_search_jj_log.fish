function _fzf_search_jj_log --description "Search the output of jj log and preview commits. Replace the current token with the selected commit hash."
    if not jj root >/dev/null 2>&1
        echo There is no jj repo in (pwd) >&2
    else
        if not set --query fzf_jj_log_template
            set -f fzf_jj_log_template 'change_id.short() ++ " " ++ committer.timestamp().format("%Y-%m-%d") ++ " " ++ description.first_line() ++ "\n"'
        end

        set -f preview_cmd 'echo {} | grep -oE "[a-z0-9]{7,}" | head -n 1 | xargs -r jj show --color=always'
        if set --query fzf_diff_highlighter
            set preview_cmd "$preview_cmd | $fzf_diff_highlighter"
        end

        set -f selected_log_lines (
            jj log -r 'all()' --color=always -T $fzf_jj_log_template | \
            _fzf_wrapper --ansi \
                --multi \
                --scheme=history \
                --prompt="JJ Log> " \
                --preview=$preview_cmd \
                --query=(commandline --current-token) \
                $fzf_jj_log_opts
        )
        if test $status -eq 0
            for line in $selected_log_lines
                set -l id (string match -r -g '([a-z0-9]{7,})' $line)[1]
                if test -n "$id"
                    set -a commit_ids $id
                end
            end
            commandline --current-token --replace (string join ' ' $commit_ids)
        end
    end

    commandline --function repaint
end
