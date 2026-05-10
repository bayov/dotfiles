# Run this configuration only on Cato machines
test "$MACHINE_PROFILE" = cato || return

# Added by OrbStack: command-line tools and integration
# This won't be added again if you remove it.
source ~/.orbstack/shell/init2.fish 2>/dev/null || :
