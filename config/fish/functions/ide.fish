function ide --description 'Open VS Code'
    if not set -q argv[1]
        "/mnt/c/Programs/VS Code/Code.exe" .
    else
        "/mnt/c/Programs/VS Code/Code.exe" $argv
    end
end
