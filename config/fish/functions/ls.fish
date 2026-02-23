# Default ls function (used by `l.fish` function)
function ls --wraps eza
    eza --long --all --icons=auto --git --time-style=long-iso $argv
end
