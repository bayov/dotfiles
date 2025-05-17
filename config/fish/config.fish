if status is-interactive
    # Abbreviations
    abbr -a !! --position anywhere --function last_history_item
    abbr -a t eza --long --tree --icons=auto --group-directories-first --git --time-style=long-iso --total-size

    # Jujutsu
    function s
        jj log --no-pager
        echo -e "\e[90m🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄\e[0m"
        jj status --no-pager
    end
    abbr -a d jj diff
    abbr -a e jj edit
    abbr -a c jj commit -m

    # Initialize prompt
    starship init fish | source
end
