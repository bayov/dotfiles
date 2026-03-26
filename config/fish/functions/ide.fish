function ide --description 'Open VS Code silently'
    if not set -q argv[1]
        command "/mnt/c/Programs/VS Code/Code.exe" . >/dev/null 2>&1 </dev/null &
    else
        command "/mnt/c/Programs/VS Code/Code.exe" $argv >/dev/null 2>&1 </dev/null &
    end
    disown
end
