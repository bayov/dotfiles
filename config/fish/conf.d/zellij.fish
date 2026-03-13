status is-interactive || return

# Ignore if not inside a Zellij session.
set --query ZELLIJ || return

set --query zellij_fish_dump_full_screen
or set --universal zellij_fish_dump_full_screen 0

function __zellij_scrollback_dump
    set -l tmpfile (command mktemp)
    set -l options
    test $zellij_fish_dump_full_screen -eq 1; and set -a options --full
    command zellij action dump-screen $options $tmpfile
    command cat $tmpfile
    command rm $tmpfile
end

function zellij_scrollback_paths --description "Prints all paths that exist in the current Zellij pane's scrollback."
    # FIXME: does not capture every true positive yet
    __zellij_scrollback_dump \
        | string match --all --regex --groups-only "(\S+[^/])" \
        | while read p
        # Only output paths that exists
        test -e $p; and echo $p
    end \
        | command sort --unique
end

function zellij_scrollback_dirs --description "Prints all dirs that exist in the current Zellij pane's scrollback."
    zellij_scrollback_paths | while read p
        test -d $p; and echo $p
    end
end

function zellij_scrollback_files --description "Prints all files that exist in the current Zellij pane's scrollback."
    zellij_scrollback_paths | while read p
        test -f $p; and echo $p
    end
end

function zellij_scrollback_urls --description "Prints all URLs that exist in the current Zellij pane's scrollback."
    set -l regexp "(https?://[^\s\"^]+)"
    __zellij_scrollback_dump \
        | string match --regex --all --groups-only $regexp \
        | awk '!x[$0]++'  # remove duplicates
end
