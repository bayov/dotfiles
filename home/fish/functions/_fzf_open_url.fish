function _fzf_open_url --description "Opens the first provided argument as a URL in a browser."
    echo (set_color blue)Opening URL: "$argv[1]"(set_color normal)
    powershell.exe Start-Process "$argv[1]"
end
