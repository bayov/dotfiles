if status is-interactive
    # Abbreviations
    abbr -a !! --position anywhere --function last_history_item

    # Aliases
    alias r=reload
    alias hm="history merge"
    alias v=vim
    alias fd=fdfind
    alias cl=claude
    alias g=gemini
    alias oc=opencode
    alias z=zellij
    alias ll=l

    # Initialize prompt
    starship init fish | source
    
    # Silence fish shell greeting
    set -gx fish_greeting  

    # Project locations for project search fzf (Alt+W)
    set -g fzf_project_paths ~/dotfiles
    set -g fzf_project_search_paths ~/projects

    # Options for directory search fzf (Alt+F)
    set -g fzf_fd_opts --hidden --follow
end

# Global aliases (even if shell is not interactive)
alias cat=batcat
alias bat=batcat
alias vim=nvim

# Environment Variables
set -gx EDITOR nvim
set -gx PNPM_HOME "/home/bayov/.local/share/pnpm"

# $PATH
fish_add_path ~/.local/bin
fish_add_path /snap/bin
fish_add_path ~/.cargo/bin
fish_add_path ~/.local/share/pnpm
fish_add_path ~/.opencode/bin
