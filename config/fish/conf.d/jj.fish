# Jujutsu specific configuration, loaded only if Jujutsu is installed
command -q jj || return

# Also, avoid loading when in Google (since we use hg instead)
string match -r '.c.googlers.com$' (hostname) &>/dev/null && return

function s
    jj log --no-pager; or return
    and echo -e "\e[38;5;242m─────────────────────────────\e[0m"

    set -l diff_output (jj diff --summary --color always | string collect); or return
    if test -n "$diff_output"
        echo -e "$diff_output"
    else
        echo -e "\e[38;5;242mWorking copy has no changes.\e[0m"
    end
end

function desc --description "Generates AI description for the current commit based on its diff"
    # get current commit git diff
    set -l diff (jj diff --git | string collect)
    test -z "$diff"; and echo "No changes found."; and return 1

    # Warn if description already exists
    set -l current_desc (jj log -r @ -n 1 --no-graph -T 'description' | string collect | string trim)
    if test -n "$current_desc"
        echo -e "\e[33mCurrent description:\e[0m"
        echo -e "\e[38;5;242m─────────────────────────────\e[0m"
        echo "$current_desc"
        echo -e "\e[38;5;242m─────────────────────────────\e[0m"
        read -l -P "Override? [Y/n] " confirm
        if test "$confirm" = n -o "$confirm" = N
            return 1
        end
    end

    # Get last 50 commits to use as example
    set -l log (jj log -r '~@' -n 50 --no-graph --template 'description ++ "--------------------------\n"' | string collect)

    # Strict prompt: raw text only, no tools
    set -l prompt "Task: Write a commit message for the provided diff.
Use the recent log for style reference.
Output ONLY the message text. No tools. No chatter.

Recent log:
--------------------------
$log

Git diff:
$diff"

    echo -e "\e[34mGenerating commit message...\e[0m"
    set -l msg (echo $prompt | env OPENCODE_DISABLE_DEFAULT_PLUGINS=true OPENCODE_DISABLE_CLAUDE_CODE=true opencode run --model opencode/trinity-large-preview-free --format json | jq -r 'select(.type == "text") | .part.text' | string collect)
    test -z "$msg"; and return 1

    jj describe -m "$msg" --quiet
    echo -e "\e[32mGenerated commit description:\e[0m"
    echo -e "\e[38;5;242m─────────────────────────────\e[0m"
    echo "$msg"
    echo -e "\e[38;5;242m─────────────────────────────\e[0m"
end

function cm --description "Generates AI description for the current commit and starts a new change"
    desc; or return
    jj new
end

function p --description "Updates bookmark `main` to track `@-` and pushes to remote"
    # Ensure this is a `jj` project and find its root directory.
    set -l root (jj root --color=always); or return 1

    # If it's a personal project, push parent commit as main git branch
    if test -f "$root/.bayov-personal-project"
        jj bookmark move main --to @-
        and jj git push
        and git checkout main
    else
        echo "Error: not personal a project (missing $root/.bayov-personal-project)"
        return 1
    end
end

abbr -a new jj new
abbr -a sync jj git fetch
abbr -a d jj diff
abbr -a e jj edit
abbr -a c --set-cursor "jj commit -m \"%\""
