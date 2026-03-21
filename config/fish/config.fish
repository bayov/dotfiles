if status is-interactive
    # Abbreviations
    abbr -a !! --position anywhere --function last_history_item
    abbr -a r reload
    abbr -a v fresh
    abbr -a ll l
    abbr -a z zellij
    abbr -a fd fdfind
    abbr -a c docker-sandbox claude --dangerously-skip-permissions
    abbr -a g docker-sandbox gemini --yolo
    abbr -a oc opencode

    # Aliases
    alias za="zellij attach default --create"
    alias zl="zellij list-sessions"
    alias zka="zellij kill-all-sessions"
    alias hm="history merge"

    # Initialize prompt
    starship init fish | source

    # Keybinds
    bind up             up-or-search
    bind down           down-or-search
    bind ctrl-h         backward-kill-word  # ctrl+h == ctrl+backspace
    bind ctrl-delete    kill-word
    bind alt-backspace  backward-kill-line
    bind alt-delete     kill-line

    # Silence fish shell greeting
    set -gx fish_greeting

    # fzf settings
    # ------------    
    set -gx FZF_DEFAULT_OPTS '
        --cycle
        --style=full
        --layout=reverse
        --height=90%
        --preview-window=wrap
        --bind="ctrl-up:preview-up+preview-up"
        --bind="ctrl-down:preview-down+preview-down"
        --bind="ctrl-page-up:preview-half-page-up"
        --bind="ctrl-page-down:preview-half-page-down"
        --bind="ctrl-alt-c:execute-silent(printf '%s' {+} | clip)"
        --bind="ctrl-y:execute-silent(printf '%s' {+} | clip)"
    '
    # Differ in previews
    set -gx fzf_diff_highlighter delta --paging=never
    # Project locations for project search fzf (Alt+W)
    set -gx fzf_project_paths ~/dotfiles
    set -gx fzf_project_search_paths ~/projects ~/workspaces
    # Options for directory search fzf (Alt+F)
    set -gx fzf_fd_opts --hidden --follow
    set -gx fzf_preview_dir_cmd "ls_with_default_args --color=always"
    # Options for jj log search fzf (Ctl+Alt+J)
    set -gx fzf_jj_log_template "one_line_no_graph"

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
alias quick_prompt="~/.local/share/quick_prompt/quick_prompt.py"

# Environment Variables
set -gx EDITOR fresh
set -gx PNPM_HOME "$HOME/.local/share/pnpm"
set -gx RUSTC_WRAPPER sccache
set -fx SCCACHE_DIR "$HOME/.cache/sccache"
set -gx SCCACHE_CACHE_SIZE "25GB"
set -gx generate_commit_message_template "$HOME/.config/resources/generate_commit_template.md"

# $PATH
fish_add_path ~/.local/bin
fish_add_path ~/.fzf/bin
fish_add_path /snap/bin
fish_add_path ~/.cargo/bin
fish_add_path ~/.local/share/pnpm
fish_add_path ~/.opencode/bin
