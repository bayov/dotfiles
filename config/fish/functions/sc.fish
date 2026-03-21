function sc --description "Run Claude in a Docker sandbox"
    set -l image_tag "claude-sc"
    set -l dockerfile_path "$HOME/.claude/Dockerfile.sc"

    set -l is_shell 0
    set -l is_rebuild 0
    set -l claude_args

    for arg in $argv
        if test "$arg" = "--rebuild"
            set is_rebuild 1
        else if test "$arg" = "--shell" -o "$arg" = "-s"
            set is_shell 1
        else
            set -a claude_args $arg
        end
    end

    if test $is_rebuild -eq 1
        if not test -f $dockerfile_path
            echo (set_color red)"Error: Dockerfile not found at $dockerfile_path. Please ensure dotfiles are deployed."(set_color normal)
            return 1
        end
        set -l actual_path (realpath $dockerfile_path)
        echo (set_color yellow)"Rebuilding Claude sandbox image ($image_tag)..."(set_color normal)
        docker build --no-cache -t $image_tag -f $actual_path (dirname $actual_path)
        or return 1
    else if not docker image inspect $image_tag >/dev/null 2>&1
        if not test -f $dockerfile_path
            echo (set_color red)"Error: Dockerfile not found at $dockerfile_path. Please ensure dotfiles are deployed."(set_color normal)
            return 1
        end
        set -l actual_path (realpath $dockerfile_path)
        echo (set_color yellow)"Building Claude sandbox image ($image_tag)..."(set_color normal)
        docker build -t $image_tag -f $actual_path (dirname $actual_path)
        or return 1
    end

    # Set up host mounts
    set -l dotfiles_dir (realpath "$HOME/dotfiles")
    set -l mounts \
        --volume (pwd):/workspace \
        --volume $HOME/.claude:/home/node/.claude \
        --volume $HOME/.claude.json:/home/node/.claude.json \
        --volume $HOME/.gitconfig:/home/node/.gitconfig:ro \
        --volume $dotfiles_dir:$dotfiles_dir:ro

    # Map essential terminal configurations to make the sandbox feel like home
    for conf in fish starship.toml jj delta eza fresh quick_prompt_models.json
        if test -e $HOME/.config/$conf
            set -a mounts --volume $HOME/.config/$conf:/home/node/.config/$conf:ro
        end
    end

    # Map quick_prompt script if it exists
    if test -d $HOME/.local/share/quick_prompt
        set -a mounts --volume $HOME/.local/share/quick_prompt:/home/node/.local/share/quick_prompt:ro
    end

    # Map ~/.config/claude-code if it exists (Claude sometimes stores data here)
    if test -d $HOME/.config/claude-code
        set -a mounts --volume $HOME/.config/claude-code:/home/node/.config/claude-code
    end

    # Run the container
    if test $is_shell -eq 1
        docker run -it --rm $mounts --workdir /workspace $image_tag fish $claude_args
    else
        docker run -it --rm $mounts --workdir /workspace $image_tag claude --dangerously-skip-permissions $claude_args
    end
end
