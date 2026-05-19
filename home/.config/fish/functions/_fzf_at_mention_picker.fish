# `@` at start of a token: open fzf file picker (via _fzf_search_directory).
# The selected path replaces the `@` (no prefix kept).
# Otherwise: insert a literal `@` so emails, git refs, etc. still work.
function _fzf_at_mention_picker
    set -l buf (commandline -c)
    if test -n "$buf"; and not string match -qr '\s$' -- "$buf"
        commandline -i '@'
        return
    end

    set -l before (commandline)
    _fzf_search_directory
    set -l after (commandline)

    # On cancel, fall back to inserting a literal `@`.
    if test "$before" = "$after"
        commandline -i '@'
    end
end
