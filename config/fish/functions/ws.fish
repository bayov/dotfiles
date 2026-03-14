function ws -d "Create a new jj workspace in ~/workspaces/"
    if test (count $argv) -eq 0
        echo "Usage: jw <workspace_name>"
        return 1
    end

    set -l ws_name $argv[1]
    set -l ws_path ~/workspaces/$ws_name

    jj workspace add $ws_path
end
