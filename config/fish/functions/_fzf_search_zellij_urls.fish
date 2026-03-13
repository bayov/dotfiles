function _fzf_search_zellij_urls --description "Search URLs in Zellij scrollback. Replace the current token with the selected URLs."
    if not set --query ZELLIJ
        echo "Not inside a Zellij session." >&2
        commandline --function repaint
        return
    end

    set -f fzf_arguments \
        --header="[Ctrl+O] Open URL" \
        --multi \
        --ansi \
        --height=50% \
        --prompt="Zellij URLs ❯ " \
        --expect=ctrl-o

    set -f fzf_output (zellij_scrollback_urls | _fzf_wrapper $fzf_arguments)

    if test $status -eq 0
        set -f key $fzf_output[1]
        set -f selected_urls $fzf_output[2..-1]

        if test "$key" = "ctrl-o"
            for url in $selected_urls
                _fzf_open_url "$url"
            end
        else
            commandline --current-token --replace -- (string escape -- $selected_urls | string join ' ')
        end
    end

    commandline --function repaint
end
