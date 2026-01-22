# Default ls function (used by `l.fish` function)
function ls_with_default_args --wraps eza
    eza --long --all --icons=auto --git --time-style=long-iso $argv
end
