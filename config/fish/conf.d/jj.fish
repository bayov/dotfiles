# Jujutsu specific configuration, loaded only if Jujutsu is installed
command -q jj || return

# Also, avoid loading when in Google (since we use hg instead)
string match -r '.c.googlers.com$' (hostname) &>/dev/null && return

function s
    jj log --no-pager; or return
    and echo -e "\e[38;5;242m────────────────────────────\e[0m"

    set -l diff_output (jj diff --summary --color always | string collect); or return
    if test -n "$diff_output"
        echo -e "$diff_output"
    else
        echo -e "\e[38;5;242mWorking copy has no changes.\e[0m"
    end
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
