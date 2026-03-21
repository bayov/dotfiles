function sc --description "Run Claude in a Docker sandbox"
    if contains -- "--rebuild" $argv
        docker-sandbox --rebuild claude --dangerously-skip-permissions (string match -v -- "--rebuild" $argv)
    else if contains -- "--shell" $argv; or contains -- "-s" $argv
        # If user explicitly asks for shell via sc, just launch the generic sandbox
        docker-sandbox
    else
        docker-sandbox claude --dangerously-skip-permissions $argv
    end
end