if status is-interactive
    # Abbreviations
    abbr -a !! --position anywhere --function last_history_item

    # Aliases
    alias r=reload
    alias v=vim
    alias fd=fdfind
    alias j=claude
    alias g=gemini
    alias z=zellij

    # Initialize prompt
    starship init fish | source
end

# Global aliases (even if shell is not interactive)
alias cat=batcat
alias bat=batcat
alias vim=nvim
