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
    # Get current commit git diff
    set -l diff (jj diff --git | string collect)
    test -z "$diff" && echo "No changes found." && return 1

    # Run my personal formatter
    format_personal_project

    # Warn if description already exists
    set -l current_desc (jj log -r @ -n 1 --no-graph -T 'description' | string collect | string trim)
    if test -n "$current_desc"
        echo_box "$current_desc" --title="Existing commit description:" # --title-color=3
        gum confirm "Override description?" --prompt.foreground=3 --selected.background=3 --padding="0 1" || return
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

    # Write prompt to a temporary file because `gum spin` executes its command
    # in a subshell, and we invoke `fish -c ...`, and passing the $prompt there
    # will be hard without escaping all `fish` syntax elements.
    #
    # This is also good for debugging (can see the last sent prompt).
    echo -e $prompt > /tmp/generate-commit-desc-prompt

    # show loader while `__generate_commit_message` is running
    set -l msg (gum spin --title "Generating commit message..." -- fish -c "quick_prompt (cat /tmp/generate-commit-desc-prompt)")
    test -z "$msg" && return 1

    jj describe -m "$msg" --quiet
    
    echo_box "$msg" --title="Generated commit description:" # --title-color=2
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
        jj bookmark move main --to @- --quiet
        and jj git push
        and git checkout main --quiet
    else
        echo "Error: not personal a project (missing $root/.bayov-personal-project)"
        return 1
    end
end

abbr -a desc jj desc
abbr -a new jj new
abbr -a sq jj squash
abbr -a d jj diff
abbr -a e jj edit
abbr -a c --set-cursor "jj commit -m \"%\""
