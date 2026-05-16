# Environment Variables
set -q DOTFILES_DIR || set -gx DOTFILES_DIR "$HOME/dotfiles"
set -gx EDITOR fresh
set -gx PNPM_HOME "$HOME/.local/share/pnpm"
set -gx RUSTC_WRAPPER sccache
set -gx SCCACHE_DIR "$HOME/.cache/sccache"
set -gx SCCACHE_CACHE_SIZE 25GB
set -gx SCCACHE_BASEDIRS "$HOME/projects:$HOME/workspaces:$HOME/dotfiles"
set -gx PYTHONPATH "$DOTFILES_DIR/pybayov/src"
set -gx BUN_INSTALL "$HOME/.bun"
set -gx CLAUDE_CODE_TMUX_TRUECOLOR 1

# Read .machine-profile file, and warn if it doesn't exist
if test -f "$DOTFILES_DIR/.machine-profile"
    set -gx MACHINE_PROFILE (string trim (cat "$DOTFILES_DIR/.machine-profile"))
else if status is-interactive
    set_color yellow
    echo "warning: $DOTFILES_DIR/.machine-profile not found" >&2
    set_color normal
end

if test "$MACHINE_PROFILE" = cato
    set -gx PLAN_DIR ~/plan
    set -gx GLOSSARY_DIR ~/plan/00_glossary
end

if test "$TERM_PROGRAM" = WarpTerminal
    set -Ua fish_features no-query-terminal
end

# Global aliases (even if shell is not interactive)
# On Debian, `bat` and `fd` ship as `batcat`/`fdfind` to avoid name conflicts;
# on macOS (brew) they keep their upstream names. Alias to whichever exists.
if command -q batcat
    alias cat=batcat
    alias bat=batcat
else if command -q bat
    alias cat=bat
end
if command -q fdfind
    alias fd=fdfind
end

# pybayov script aliases
alias bayov-format="env -u VIRTUAL_ENV uv run --project $DOTFILES_DIR/pybayov python -m pybayov.format"
alias bayov-test="env -u VIRTUAL_ENV uv run --project $DOTFILES_DIR/pybayov python -m pybayov.test"
alias echo-box="env -u VIRTUAL_ENV uv run --project $DOTFILES_DIR/pybayov python -m pybayov.echo_box"
alias spinner="env -u VIRTUAL_ENV uv run --project $DOTFILES_DIR/pybayov python -m pybayov.spinner"
alias quick-prompt="env -u VIRTUAL_ENV uv run --project $DOTFILES_DIR/pybayov python -m pybayov.quick_prompt"
alias docker-sandbox="env -u VIRTUAL_ENV uv run --project $DOTFILES_DIR/pybayov python -m pybayov.docker_sandbox"
alias generate-commit-description="env -u VIRTUAL_ENV uv run --project $DOTFILES_DIR/pybayov python -m pybayov.generate_commit_description"

alias precommit=bayov-format
alias prepush="bayov-test --quiet --show-failures"

# $PATH
fish_add_path ~/.local/bin
fish_add_path ~/.fzf/bin
fish_add_path /snap/bin
fish_add_path ~/.cargo/bin
fish_add_path ~/.local/share/pnpm
fish_add_path ~/.opencode/bin
fish_add_path ~/go/bin
fish_add_path $BUN_INSTALL/bin

# Configuration that applies only for interactive shell sessions
if status is-interactive
    abbr -a !! --position anywhere --function last_history_item
    abbr -a r reload
    abbr -a ll l
    abbr -a z zellij
    abbr -a zl zellij list-sessions
    abbr -a zka zellij kill-all-sessions
    abbr -a c claude
    abbr -a g gemini --yolo
    abbr -a oc opencode
    abbr -a lmd glow --width 160

    # Aliases
    alias claude="claude --dangerously-skip-permissions"
    alias za="zellij attach default --create"
    alias hm="history merge"
    alias v="$EDITOR"

    # Initialize prompt
    type -q starship && starship init fish | source

    # Keybinds
    bind up up-or-search
    bind down down-or-search
    bind ctrl-h backward-kill-word # ctrl+h == ctrl+backspace
    bind ctrl-delete kill-word
    bind alt-backspace backward-kill-line
    bind alt-delete kill-line
    # Double-Escape clears the current command line (like Ctrl+C).
    # `fish_sequence_key_delay_ms` widens the wait between sequence keys only
    # (it does not affect Alt/Esc disambiguation, which uses fish_escape_delay_ms).
    set -g fish_sequence_key_delay_ms 300
    bind escape,escape 'commandline -r ""'

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
        --bind="alt-q:abort"
    '
    # Differ in previews
    set -gx fzf_diff_highlighter delta --paging=never
    # Project roots for project search fzf (Alt+W) - recursively finds dirs containing .git or .jj
    set -gx fzf_project_roots $DOTFILES_DIR ~/projects ~/projects-archon ~/workspaces ~/notes
    # Options for directory search fzf (Alt+F)
    set -gx fzf_fd_opts --hidden --follow
    set -gx fzf_preview_dir_cmd "ls_with_default_args --color=always"
    # Options for jj log search fzf (Ctl+Alt+J)
    set -gx fzf_jj_log_template one_line_no_graph

    # gum settings
    # ------------
    # confirm
    set -gx GUM_CONFIRM_UNSELECTED_BACKGROUND "#0f0f12"
    set -gx GUM_CONFIRM_SELECTED_BACKGROUND 4
    set -gx GUM_CONFIRM_SELECTED_FOREGROUND 16
    set -gx GUM_CONFIRM_PROMPT_FOREGROUND 4

    if set -q DOCKER_SANDBOX
        # Disable frequent history syncing in sandbox to avoid lag on shared volumes
        # (This is especially slow between keystrokes)
        set -g fish_history_sync_on_keystroke 0
        set -g fish_handle_history_sync 0

        # Override starship entirely for speed in the sandbox
        # Starship custom commands (like jj-starship) are very slow over Docker mounts
        function fish_prompt
            set_color red
            echo -n "[SANDBOX]"
            set_color normal
            echo -n " ❯ "
        end
        function fish_right_prompt
            set_color blue
            echo -n (prompt_pwd)" "
            set_color normal
        end
        function fish_title
        end

        # Disable slow completion features (like descriptions via apropos)
        set -gx FISH_COMPLETION_NO_DESCRIPTIONS 1
    end
end
