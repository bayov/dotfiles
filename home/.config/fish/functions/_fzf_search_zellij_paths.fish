function _fzf_search_zellij_paths --description "Search paths in Zellij scrollback. Replace the current token with the selected paths."
    if not set --query ZELLIJ
        echo "Not inside a Zellij session." >&2
        commandline --function repaint
        return
    end

    set -f fzf_arguments \
        --header="[Alt+E] Edit file" \
        --multi \
        --ansi \
        --height=50% \
        --prompt="Zellij Paths ❯ " \
        --preview='_fzf_preview_file {}' \
        --expect=alt-e

    set -f fzf_output (zellij_scrollback_paths | fzf $fzf_arguments)
    set -f fzf_status $status

    if test $fzf_status -eq 0
        set -f key $fzf_output[1]
        set -f selected_paths $fzf_output[2..]
        if test "$key" = alt-e
            $EDITOR $selected_paths
        else
            commandline --current-token --replace -- (string escape -- $selected_paths | string join ' ')
        end
    end

    commandline --function repaint
end
