function imgcat
    if set -q WSL_DISTRO_NAME
        wezterm.exe imgcat (wslpath -w $argv)
    else
        wezterm imgcat $argv
    end
end
