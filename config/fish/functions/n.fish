function n --description "Note-taking: open or create a note in \$personal_notes_default_dir"
    if test (count $argv) -eq 0
        echo "Usage: n <note.md>"
        return 1
    end

    set notes_dir (test -n "$personal_notes_default_dir"; and echo $personal_notes_default_dir; or echo ~/dotfiles/notes)
    set note_path $notes_dir/$argv[1]

    if not test -d $notes_dir
        mkdir -p $notes_dir
    end

    if not test -f $note_path
        touch $note_path
    end

    $EDITOR $note_path
end
