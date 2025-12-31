# Run this configuration only in Google
string match -r '.c.googlers.com$' (hostname) &>/dev/null || return

# Run gcert on login if less than 60m remain
if status --is-interactive
  gcertstatus --check_remaining=60m >/dev/null 2>&1
  or gcert -s
end

# Run gcert when less than 60m remain in long sessions
source_google_fish_package autogcert

# Autocompletion for the following
source_google_fish_package hi
source_google_fish_package citc_prompt
source_google_fish_package benchy
source_google_fish_package banshee

# hg abbreviations
function s
    hg --pager=never xl
    and echo -e "\e[90m---------------------\e[0m"
    and hg --pager=never status
end
abbr -a sync hg sync
abbr -a am hg am
abbr -a uc hg uc
abbr -a d hg diff
abbr -a e hg checkout
abbr -a c --set-cursor "hg commit -m \"%\""

# misc abbreviations
function prodspec --wraps=/google/bin/releases/rollouts/prodspec/prodspec
    /google/bin/releases/rollouts/prodspec/prodspec $argv
end
function gemini --wraps=/google/bin/releases/gemini-cli/tools/gemini
    /google/bin/releases/gemini-cli/tools/gemini $argv
end
