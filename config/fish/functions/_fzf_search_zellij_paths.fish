function _fzf_search_zellij_paths --description "Search paths in Zellij scrollback. Replace the current token with the selected paths."
    if not set --query ZELLIJ
        echo "Not inside a Zellij session." >&2
        commandline --function repaint
        return
    end

    set -f fzf_arguments \
        --header="[Ctrl+E] Edit file" \
        --multi \
        --ansi \
        --height=50% \
        --prompt="Zellij Paths ❯ " \
        --preview='_fzf_preview_file {}' \
        --bind="ctrl-e:execute($EDITOR {})+abort"

    set -f selected_paths (zellij_scrollback_paths | _fzf_wrapper $fzf_arguments)

    if test $status -eq 0
        commandline --current-token --replace -- (string escape -- $selected_paths | string join ' ')
    end

    commandline --function repaint
end
