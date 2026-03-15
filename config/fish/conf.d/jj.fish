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
    set -l current_desc (jj log -r @ -n 1 --no-graph -T 'description' | string trim | string collect)
    if test -n "$current_desc"
        echo_box "$current_desc" --title="Existing commit description:" # --title-color=3
        gum confirm "Override description?" --prompt.foreground=3 --selected.background=3 --padding="0 1" || return
    end

    # Get last 50 commits to use as example
    # set -l log (jj log -r '~@' -n 50 --no-graph --template 'description ++ "--------------------------\n"' | string collect)

    # Load prompt from template
    set -l prompt (cat "$generate_commit_message_template" | string replace "{{diff}}" "$diff" | string collect)

    # Write prompt to a temporary file because `gum spin` executes its command
    # in a subshell, and we invoke `fish -c ...`, and passing the $prompt there
    # will be hard without escaping all `fish` syntax elements.
    #
    # This is also good for debugging (can see the last sent prompt).
    echo -e $prompt > /tmp/generate-commit-desc-prompt

    # show loader while `__generate_commit_message` is running
    set -l msg (gum spin --title "generating commit message" -- fish -c "cat /tmp/generate-commit-desc-prompt | quick_prompt" | string trim | string collect)
    test -z "$msg" && return 1

    jj describe -m "$msg" --quiet
    
    echo_box "$msg" --title="Generated commit description:" --width 100 # --title-color=2
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
        test_personal_project || return $status

        # Check if `main` bookmark is already at parent (`@-`)
        if not test -z (jj log -r "main & @-" -T 1 --no-graph)
            echo (set_color yellow)Warning: (set_color normal)Nothing to push
            return 1
        end

        jj bookmark move main --to @- --quiet
        and jj git push --quiet
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
abbr -a c --set-cursor "jj commit -m \"%\""
