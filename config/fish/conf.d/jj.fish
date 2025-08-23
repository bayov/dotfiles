# Jujutsu specific configuration, loaded only if Jujutsu is installed
command -q jj || return

# Also, avoid laoding when in Google (since we use hg instead)
string match -r '.c.googlers.com$' (hostname) &>/dev/null && return

function s
    jj log --no-pager
    and echo -e "\e[38;5;242m────────────────────────────\e[0m"

    set -l diff_output (jj diff --summary --color always | string collect)
    if test -n "$diff_output"
        echo -e "$diff_output"
    else
        echo -e "\e[38;5;242mWorking copy has no changes.\e[0m"
    end
end

function push-main --description "Updates bookmark `main` to track `@-` and pushes to remote"
    jj bookmark move main --to @-
    and jj git push
end

abbr -a new jj new
abbr -a sync jj git fetch
abbr -a d jj diff
abbr -a e jj edit
abbr -a c --set-cursor "jj commit -m \"%\""
