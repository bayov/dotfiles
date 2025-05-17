# Jujutsu specific configuration, loaded only if Jujutsu is installed
command -q jj || return

function s
    jj log --no-pager
    echo -e "\e[90m🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄🞄\e[0m"
    jj status --no-pager
end
function push-main --description "Updates bookmark `main` to track `@-` and pushes to remote"
    jj bookmark move main --to @-
    jj git push
end
abbr -a d jj diff
abbr -a e jj edit
abbr -a c --set-cursor "jj commit -m \"%\""