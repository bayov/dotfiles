function ws -d "Manage jj workspaces in ~/workspaces/"
    set -l cmd $argv[1]
    set -l name $argv[2]
    set -l base_dir ~/workspaces
    set -l backup_dir /tmp/deleted-workspaces

    switch "$cmd"
        case add
            if test -z "$name"
                echo "Usage: ws add <workspace_name>"
                return 1
            end

            set -l workspace_dir "$base_dir/$name"
            if jj workspace add "$workspace_dir"
                if set -q ZELLIJ
                    set -l layout_file "/tmp/ws-zellij-layout.$name.kdl"
                    begin
                        echo "layout {"
                        echo "    pane split_direction=\"vertical\" {"
                        echo "        pane cwd=\"$workspace_dir\""
                        echo "        pane cwd=\"$workspace_dir\""
                        echo "    }"
                        echo "    pane size=1 borderless=true {"
                        echo "        plugin location=\"zellij:compact-bar\""
                        echo "    }"
                        echo "}"
                    end > "$layout_file"
                    zellij action new-tab --name "ws/$name" --layout "$layout_file"
                end
            end

        case list
            jj workspace list

        case remove rm
            if test -z "$name"
                echo "Usage: ws remove <workspace_name>"
                return 1
            end

            if jj workspace forget "$name"
                set -l timestamp (date "+%Y-%m-%d.%H%M%S")
                set -l target_path "$backup_dir/$timestamp.$name"

                mkdir -p "$backup_dir"

                if test -d "$base_dir/$name"
                    mv "$base_dir/$name" "$target_path"
                    echo "Workspace '$name' forgotten and moved to $target_path"
                else
                    echo "Workspace '$name' forgotten, but directory $base_dir/$name was not found."
                end
            end

        case rm-current
            set -l real_base (realpath $base_dir)
            set -l real_pwd (realpath (pwd))

            if string match -q "$real_base/*" "$real_pwd"
                set -l rel_path (string replace "$real_base/" "" "$real_pwd")
                set -l ws_name (string split -m 1 / "$rel_path")[1]

                if test -n "$ws_name"
                    ws remove "$ws_name"
                    if set -q ZELLIJ
                        zellij action close-tab
                    end
                end
            else
                echo "Current directory is not within $base_dir"
                return 1
            end

        case '*'
            echo "Usage: ws {add|list|remove|rm-current} [name]"
            return 1
    end
end

# --- Completions ---
# Disable file completions for the main command
complete -c ws -f 

# Subcommands
complete -c ws -n "__fish_use_subcommand" -a add -d "Add a new workspace"
complete -c ws -n "__fish_use_subcommand" -a list -d "List all workspaces"
complete -c ws -n "__fish_use_subcommand" -a "remove rm" -d "Forget and archive a workspace"
complete -c ws -n "__fish_use_subcommand" -a rm-current -d "Remove current workspace and close Zellij tab"

# Dynamic completion for 'remove' (shows existing workspaces)
# This assumes 'jj workspace list' output format is 'name: path'
complete -c ws -n "__fish_seen_subcommand_from remove rm" -a "(jj workspace list | cut -d: -f1)"
