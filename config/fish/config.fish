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

    # Up/Down do a prefix reverse search
    bind \e\[A history-prefix-search-backward
    bind \e\[B history-prefix-search-forward

    # Silence fish shell greeting
    set -gx fish_greeting

    # fzf settings
    # ------------
    # Differ in previews
    set -g fzf_diff_highlighter delta --paging=never
    # Project locations for project search fzf (Alt+W)
    set -g fzf_project_paths ~/dotfiles
    set -g fzf_project_search_paths ~/projects
    # Options for directory search fzf (Alt+F)
    set -g fzf_fd_opts --hidden --follow
    set -g fzf_preview_dir_cmd "ls_with_default_args --color=always"
    # Options for jj log search fzf (Ctl+Alt+J)
    set -g fzf_jj_log_template "one_line_no_graph"

    # gum settings
    # ------------
    # confirm
    set -gx GUM_CONFIRM_UNSELECTED_BACKGROUND "#0f0f12"
    set -gx GUM_CONFIRM_SELECTED_BACKGROUND 4
    set -gx GUM_CONFIRM_SELECTED_FOREGROUND 16
    set -gx GUM_CONFIRM_PROMPT_FOREGROUND 4
    # spin
    set -gx GUM_SPIN_SPINNER points
    set -gx GUM_SPIN_TITLE_FOREGROUND 4
    set -gx GUM_SPIN_SPINNER_FOREGROUND 4
    set -gx GUM_SPIN_PADDING "0 1"
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

