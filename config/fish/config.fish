if status is-interactive
    # Abbreviations
    abbr -a r reload
    abbr -a !! --position anywhere --function last_history_item
    abbr -a v vim
    abbr -a fd fdfind
    abbr -a z zellij

    # Initialize prompt
    starship init fish | source
end

# Aliases
alias cat=batcat
alias bat=batcat
alias vim=nvim
