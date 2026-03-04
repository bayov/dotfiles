if string match -qr '.c.googlers.com$' (hostname)
    set -g use_google_config true
else
    set -g use_google_config false
end

# Run this configuration only in Google
test "$use_google_config" = "true" || return

# Run gcert on login if less than 60m remain
#if status --is-interactive
#  gcertstatus --check_remaining=60m >/dev/null 2>&1
#  or gcert -s
#end

# Run gcert when less than 60m remain in long sessions
source_google_fish_package autogcert

# Default to DBIP with Blaze
set -gx REPLACE_BLAZE_WITH_DBIP 1

# Autocompletion for the following
source_google_fish_package hi
source_google_fish_package citc_prompt
source_google_fish_package benchy
source_google_fish_package banshee
source_google_fish_package jj

# jj abbreviations
abbr -a sync jj sync
abbr -a u jj upload
function sfu
    jj squash && jj fix && jj upload && jj edit @-
end

# hg abbreviations
alias hg=chg
# `hg` aliases disabled, trying out `jj` (conf.d/jj.fish)
function hgs
    hg --pager=never xl
    and echo -e "\e[90m---------------------\e[0m"
    and hg --pager=never status
end
abbr -a hgsync hg sync
abbr -a hgam hg am
abbr -a hguc hg uc
abbr -a hge hg checkout
abbr -a hgc --set-cursor "hg commit -m \"%\""

# misc abbreviations
function prodspec --wraps=/google/bin/releases/rollouts/prodspec/prodspec
    /google/bin/releases/rollouts/prodspec/prodspec $argv
end
function gemini --wraps=/google/bin/releases/gemini-cli/tools/gemini
    /google/bin/releases/gemini-cli/tools/gemini $argv
end
