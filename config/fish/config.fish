if status is-interactive
    # Abbreviations
    abbr -a r reload
    abbr -a !! --position anywhere --function last_history_item
    abbr -a t eza --long --tree --icons=auto --group-directories-first --git --time-style=long-iso --total-size
    abbr -a fd fdfind
    abbr -a z zellij

    # Initialize prompt
    starship init fish | source
end
