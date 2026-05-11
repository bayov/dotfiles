function rm_workspace -d "Move ~/workspaces/<name> to /tmp/deleted-workspaces"
    uv run python -m pybayov.cato.rm_workspace $argv
    or return $status

    # cd out if we were inside the removed workspace
    set name $argv[1]
    if test -z "$name"
        set name (basename (pwd))
    end
    if string match -q "*/workspaces/$name*" (pwd)
        cd ~
    end
end
