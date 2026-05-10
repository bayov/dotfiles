function new_workspace -d "Clone haka into ~/workspaces/<name>, bootstrap, and cd into it"
    if test (count $argv) -lt 1
        echo "Usage: new_workspace <name> [--no-setup]" >&2
        return 1
    end

    uv run python -m pybayov.cato.new_workspace $argv
    or return $status

    cd ~/workspaces/$argv[1]
end
