function clip --description 'Cross-platform clipboard copy'
    set -l os (uname)

    # Detect environment and execute appropriate copy command
    if test "$os" = Darwin
        # macOS
        pbcopy
    else if uname -r | string match -qi "*microsoft*"
        # WSL (Windows Subsystem for Linux)
        cat | clip.exe
    else if type -q wl-copy
        # Wayland (Modern Linux)
        wl-copy
    else if type -q xclip
        # X11 (Traditional Linux)
        xclip -selection clipboard
    else if type -q xsel
        # Alternative X11
        xsel --clipboard --input
    else
        echo "Error: No clipboard utility found. Please install xclip, xsel, or wl-copy." >&2
        return 1
    end
end
