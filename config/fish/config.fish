if status is-interactive
    # Abbreviations
    abbr -a reload source ~/.config/fish/**/*.fish
    abbr -a t eza --long --tree --icons=auto --group-directories-first --git --time-style=long-iso --total-size
    abbr -a ll eza --long --all --icons=auto --git --time-style=long-iso

    # Initialize prompt
    starship init fish | source
end
