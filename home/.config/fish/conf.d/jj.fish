# Jujutsu specific configuration, loaded only if Jujutsu is installed
command -q jj || return

function s
    jj log --no-pager || return
    and echo -e "\e[38;5;242m─────────────────────────────\e[0m"

    set -l diff_output (jj diff --summary --color always | string collect) || return
    if test -n "$diff_output"
        echo -e "$diff_output"
    else
        echo -e "\e[38;5;242mWorking copy has no changes.\e[0m"
    end
end

function gm --description "Generates AI description for the current commit based on its diff"
    # Run my personal formatter
    precommit || return $status

    # Warn if description already exists
    set -l current_desc (jj log -r @ -n 1 --no-graph -T 'description' | string trim | string collect)
    if test -n "$current_desc"
        echo_box --content="$current_desc" --title="Existing Commit Description" --title-fg=3
        gum confirm "Override description?" --prompt.foreground=3 --selected.background=3 --padding="0 1" || return
    end

    set -l result (spinner --title "generating commit description" -- generate_commit_description | string collect)
    set -l error_msg (echo $result | jq -r '.error // empty' 2>/dev/null)
    if test -n "$error_msg"
        echo $error_msg
        return 1
    end
    test -n "$result" || return 1
    set -l output_file (echo $result | jq -r '.prompt_output_file')
    set -l msg (cat $output_file | string collect)
    test -n "$msg" || return 1

    jj describe -m "$msg" --quiet

    echo_box --content="$msg" --title="Generated Commit Description" --width 100
end

function cm --description "Generates AI description for the current commit and starts a new change"
    gm || return
    jj new --quiet
end

function p --description "Updates bookmark `main` to track `@-` and pushes to remote"
    # Ensure this is a `jj` project and find its root directory.
    set -l root (jj root --color=always) || return

    # If it's a personal project, push parent commit as main git branch
    if test -f "$root/.bayov-personal-project"
        prepush || return $status
        printf "\n"
        printf "\n"
        printf "\n"
        printf "\n"

        # Check if `main` bookmark is already at parent (`@-`)
        if not test -z (jj log -r "main & @-" -T 1 --no-graph)
            echo (set_color yellow)Warning: (set_color normal)Nothing to push
            return 1
        end

        jj bookmark move main --to @- --quiet

        and spinner --title "pushing main" -- jj git push --quiet
        and echo (set_color green) ✓ pushed main(set_color normal)
        and echo
        and begin
            if command git rev-parse --is-inside-work-tree >/dev/null 2>&1
                git checkout main --quiet
            end
        end
    else
        echo "Error: not personal a project (missing $root/.bayov-personal-project)"
        return 1
    end
end

abbr -a desc jj desc
abbr -a new jj new
abbr -a sq jj squash
abbr -a d jj diff -w
abbr -a e jj edit
abbr -a rebase jj rebase
abbr -a cmm --set-cursor "jj commit -m \"%\""
