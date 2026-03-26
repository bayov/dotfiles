function ide --description 'Open current directory in VS Code'
    if not set -q argv[1]
        code --new-window . >/dev/null 2>&1 </dev/null &
    else
        code --new-window $argv >/dev/null 2>&1 </dev/null &
    end
    disown
end
