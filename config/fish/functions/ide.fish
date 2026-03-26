function ide --description 'Open VS Code in WSL mode silently'
    if not set -q argv[1]
        code . >/dev/null 2>&1 </dev/null &
    else
        code $argv >/dev/null 2>&1 </dev/null &
    end
    disown
end
