function t --wraps=eza
    eza --long --tree --icons=auto --group-directories-first --git --time-style=long-iso --total-size $argv
end
