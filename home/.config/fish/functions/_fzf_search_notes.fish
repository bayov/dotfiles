function _fzf_search_notes --description "Search notes and open or create one in \$EDITOR"
    set -f notes_dir (test -n "$personal_notes_default_dir"; and echo $personal_notes_default_dir; or echo ~/notes)

    if not test -d $notes_dir
        mkdir -p $notes_dir
    end

    set -f result (ls $notes_dir | fzf \
        --prompt="Notes ❯ " \
        --print-query \
        --preview="_fzf_preview_file $notes_dir/{}")
    set -f fzf_status $status

    if test $fzf_status -eq 0
        note $result[2]
    else if test $fzf_status -eq 1
        # No match — use query as new note name
        if test -n "$result[1]"
            note $result[1]
        end
    end

    commandline --function repaint
end
