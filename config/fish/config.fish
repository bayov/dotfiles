if status is-interactive
    # Abbreviations
    abbr -a !! --position anywhere --function last_history_item

    # Aliases
    alias r=reload
    alias v=vim
    alias fd=fdfind
    alias cl=claude
    alias g=gemini
    alias oc=opencode
    alias z=zellij

    # Initialize prompt
    starship init fish | source

    # Color timestamps in functions/_fzf_search_history.fish.
    # Note: I had to remove the unicode pipe (│) from the `--show-time` history
    # flag as I wanted to color it grey as well, and I also had to add
    set -gx fzf_history_time_format (set_color 666)"%Y-%m-%d %H:%M │ "(set_color normal)
    set -gx fzf_history_opts --ansi  # To add support of ansi color escape codes

    # Allow shift-delete to remove history entries
    set -gx fzf_history_deleted_backup_file ~/.deleted-history.txt
    set -a fzf_history_opts --bind "shift-delete:execute-silent(printf '%s\n' {} | fish -c fzf_history_delete)+reload(fish -c 'builtin history --null --show-time=\$fzf_history_time_format')"
end

# Global aliases (even if shell is not interactive)
alias cat=batcat
alias bat=batcat
alias vim=nvim

# Environment Variables
set -gx EDITOR nvim
set -gx fish_greeting  # Silence fish shell greeting
set -gx fzf_fd_opts --hidden --follow
set -gx PNPM_HOME "/home/bayov/.local/share/pnpm"

# $PATH
fish_add_path ~/.local/bin
fish_add_path /snap/bin
fish_add_path ~/.cargo/bin
fish_add_path ~/.local/share/pnpm
fish_add_path ~/.opencode/bin
