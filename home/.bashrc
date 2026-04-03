# ~/.bashrc: executed by bash(1) for non-login shells.

# ── Environment Variables ──────────────────────────────────────────────────────
[ -z "$DOTFILES_DIR" ] && export DOTFILES_DIR="$HOME/dotfiles"
export EDITOR=fresh
export PNPM_HOME="$HOME/.local/share/pnpm"
export RUSTC_WRAPPER=sccache
export SCCACHE_DIR="$HOME/.cache/sccache"
export SCCACHE_CACHE_SIZE=25GB
export SCCACHE_BASEDIRS="$HOME/projects:$HOME/workspaces:$HOME/dotfiles"
export PYTHONPATH="$DOTFILES_DIR/pybayov/src"

# ── PATH ───────────────────────────────────────────────────────────────────────
_prepend_path() { [[ ":$PATH:" != *":$1:"* ]] && export PATH="$1:$PATH"; }
_prepend_path "$HOME/.local/bin"
_prepend_path "$HOME/.fzf/bin"
_prepend_path "/snap/bin"
_prepend_path "$HOME/.cargo/bin"
_prepend_path "$HOME/.local/share/pnpm"
_prepend_path "$HOME/.opencode/bin"
unset -f _prepend_path

# ── Global Aliases ─────────────────────────────────────────────────────────────
alias cat=batcat
alias bat=batcat

# pybayov script aliases
alias bayov-format="uv run python -m pybayov.format"
alias bayov-test="uv run python -m pybayov.test"
alias echo-box="uv run python -m pybayov.echo_box"
alias spinner="uv run python -m pybayov.spinner"
alias quick-prompt="uv run python -m pybayov.quick_prompt"
alias docker-sandbox="uv run python -m pybayov.docker_sandbox"
alias generate-commit-description="uv run python -m pybayov.generate_commit_description"

alias precommit=bayov-format
alias prepush="bayov-test --quiet --show-failures"

# ── Interactive-only ───────────────────────────────────────────────────────────
case $- in
*i*) ;;
*) return ;;
esac

# History
HISTCONTROL=ignoreboth
HISTSIZE=10000
HISTFILESIZE=20000
shopt -s histappend
shopt -s checkwinsize

# lesspipe
[ -x /usr/bin/lesspipe ] && eval "$(SHELL=/bin/sh lesspipe)"

# Prompt
case "$TERM" in
xterm-color | *-256color) color_prompt=yes ;;
esac
if [ "$color_prompt" = yes ]; then
	PS1='\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ '
else
	PS1='\u@\h:\w\$ '
fi
unset color_prompt

# ls colors
if [ -x /usr/bin/dircolors ]; then
	test -r ~/.dircolors && eval "$(dircolors -b ~/.dircolors)" || eval "$(dircolors -b)"
	alias ls='ls --color=auto'
	alias grep='grep --color=auto'
fi

# Bash completion
if ! shopt -oq posix; then
	if [ -f /usr/share/bash-completion/bash_completion ]; then
		. /usr/share/bash-completion/bash_completion
	elif [ -f /etc/bash_completion ]; then
		. /etc/bash_completion
	fi
fi

alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'
alias za="zellij attach default --create"
alias hm="history -n" # merge history from other sessions
alias v="$EDITOR"
alias fd=fdfind
alias lmd="glow --width 160"

# Starship prompt
type -P starship &>/dev/null && eval "$(starship init bash)"

# fzf
if [ -f ~/.fzf/shell/key-bindings.bash ]; then
	. ~/.fzf/shell/key-bindings.bash
fi
if [ -f ~/.fzf/shell/completion.bash ]; then
	. ~/.fzf/shell/completion.bash
fi

export FZF_DEFAULT_OPTS='
    --cycle
    --style=full
    --layout=reverse
    --height=90%
    --preview-window=wrap
    --bind="ctrl-up:preview-up+preview-up"
    --bind="ctrl-down:preview-down+preview-down"
    --bind="ctrl-page-up:preview-half-page-up"
    --bind="ctrl-page-down:preview-half-page-down"
    --bind="ctrl-alt-c:execute-silent(printf \"%s\" {+} | clip)"
    --bind="ctrl-y:execute-silent(printf \"%s\" {+} | clip)"
'

export fzf_diff_highlighter="delta --paging=never"
export fzf_project_paths="$DOTFILES_DIR"
export fzf_project_search_paths="$HOME/projects $HOME/workspaces"
export fzf_fd_opts="--hidden --follow"
export fzf_preview_dir_cmd="ls_with_default_args --color=always"
export fzf_jj_log_template=one_line_no_graph

# gum settings
export GUM_CONFIRM_UNSELECTED_BACKGROUND="#0f0f12"
export GUM_CONFIRM_SELECTED_BACKGROUND=4
export GUM_CONFIRM_SELECTED_FOREGROUND=16
export GUM_CONFIRM_PROMPT_FOREGROUND=4

# Docker sandbox overrides
if [ -n "$DOCKER_SANDBOX" ]; then
	PS1='\[\033[0;31m\][SANDBOX]\[\033[0m\] ❯ '
	export FISH_COMPLETION_NO_DESCRIPTIONS=1
fi
