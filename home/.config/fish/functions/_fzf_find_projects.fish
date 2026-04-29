function _fzf_find_projects --description "Recursively find project roots (dirs with .git or .jj), not recursing into found projects"
    set -f fd_cmd (command -v fdfind || command -v fd || echo "fd")

    for dir in $argv
        if test -d "$dir/.git" -o -d "$dir/.jj"
            echo $dir
        else
            set -l subdirs ($fd_cmd --hidden --no-ignore --max-depth 1 --min-depth 1 --type d . "$dir" 2>/dev/null)
            if set -q subdirs[1]
                _fzf_find_projects $subdirs
            end
        end
    end
end
