status is-interactive || return

# Ignore if not inside a Zellij session.
set --query ZELLIJ || return

set --query zellij_fish_dump_full_screen
or set --universal zellij_fish_dump_full_screen 0

function __zellij_scrollback_dump
    set -l options
    test $zellij_fish_dump_full_screen -eq 1; and set -a options --full
    command zellij action dump-screen $options
end

function zellij_scrollback_paths --description "Prints all paths that exist in the current Zellij pane's scrollback."
    # FIXME: does not capture every true positive yet
    __zellij_scrollback_dump \
        | string match --all --regex --groups-only '((?:/|~|\.\.?/|[\w._-]+/)\S*[^/ \t\n\r"\'\(\)\[\]\{\}])' \
        | while read p
        # Only output paths that exists
        # Expand ~ to $HOME because test -e doesn't do it for variables
        set -l expanded_p (string replace -r '^~' "$HOME" "$p")
        test -e "$expanded_p"; and echo "$p"
    end \
        | tac \
        | awk '!x[$0]++' # dedupe
end

function zellij_scrollback_urls --description "Prints all URLs that exist in the current Zellij pane's scrollback."
    set -l regexp "(https?://[^\s\"^]+)"
    __zellij_scrollback_dump \
        | string match --regex --all --groups-only $regexp \
        | tac \
        | awk '!x[$0]++' # dedupe
end
