function echo_box -d "Print text in a gum-styled box"
    argparse h/help 't/title=' 'c/title-color=' 'w/width=' -- $argv
    or return

    set -l default_title_color "#666666"
    set -l default_width 80

    if set -q _flag_h
        echo "Usage: echo_box [TEXT] [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  -t, --title=STR          Set box title"
        echo "  -c, --title-color=COLOR  Set title color (default: $default_title_color)"
        echo "  -w, --width=INT          Set box width (default: $default_width)"
        echo "  -h, --help               Show help"
        return 0
    end

    set -l text $argv[1]
    set -l t_color (set -q _flag_title_color; and echo $_flag_title_color; or echo "$default_title_color")
    set -l width (set -q _flag_w; and echo $_flag_w; or echo "$default_width")

    set -l content "$text"
    if set -q _flag_t
        set content (gum style "$_flag_t" --italic --foreground=$t_color)"\n$text"
    end

    echo -e $content | gum style --no-strip-ansi --border hidden --background "#0f0f12" --padding "1 2" --margin "0 0" --width $width
end
