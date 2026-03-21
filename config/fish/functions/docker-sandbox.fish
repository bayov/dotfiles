function docker-sandbox --description "Run a command or shell in a general-purpose Docker sandbox"
    set -l image_tag "sandbox-env"
    set -l dockerfile_path (realpath "$HOME/dotfiles/config/docker/Dockerfile.sandbox")

    set -l is_rebuild 0
    set -l sandbox_args

    for arg in $argv
        if test "$arg" = "--rebuild"
            set is_rebuild 1
        else
            set -a sandbox_args $arg
        end
    end

    if test $is_rebuild -eq 1
        if not test -f $dockerfile_path
            echo (set_color red)"Error: Dockerfile not found at $dockerfile_path."(set_color normal)
            return 1
        end
        echo (set_color yellow)"Rebuilding sandbox image ($image_tag)..."(set_color normal)
        docker build --no-cache -t $image_tag -f $dockerfile_path (dirname $dockerfile_path)
        or return 1
    else if not docker image inspect $image_tag >/dev/null 2>&1
        if not test -f $dockerfile_path
            echo (set_color red)"Error: Dockerfile not found at $dockerfile_path."(set_color normal)
            return 1
        end
        echo (set_color yellow)"Building sandbox image ($image_tag)..."(set_color normal)
        docker build -t $image_tag -f $dockerfile_path (dirname $dockerfile_path)
        or return 1
    end

    # Set up host mounts
    set -l dotfiles_dir (realpath "$HOME/dotfiles")
    set -l mounts \
        --volume (pwd):/workspace \
        --volume $HOME/.claude:/home/sandbox/.claude \
        --volume $HOME/.claude.json:/home/sandbox/.claude.json \
        --volume $HOME/.gitconfig:/home/sandbox/.gitconfig:ro \
        --volume $dotfiles_dir:$dotfiles_dir:ro

    # Map essential terminal configurations to make the sandbox feel like home
    for conf in fish starship.toml jj delta eza fresh quick_prompt_models.json
        if test -e $HOME/.config/$conf
            set -a mounts --volume $HOME/.config/$conf:/home/sandbox/.config/$conf
        end
    end

    # Map quick_prompt script if it exists
    if test -d $HOME/.local/share/quick_prompt
        set -a mounts --volume $HOME/.local/share/quick_prompt:/home/sandbox/.local/share/quick_prompt
    end

    # Map ~/.config/claude-code if it exists
    if test -d $HOME/.config/claude-code
        set -a mounts --volume $HOME/.config/claude-code:/home/sandbox/.config/claude-code
    end

    # Pass the ssh-agent socket if it exists
    set -l env_vars
    if test -n "$SSH_AUTH_SOCK"
        set -a mounts --volume $SSH_AUTH_SOCK:/tmp/ssh-agent.sock
        set -a env_vars --env SSH_AUTH_SOCK=/tmp/ssh-agent.sock
    end

    # Run the container
    if test (count $sandbox_args) -eq 0
        docker run -it --rm $mounts $env_vars --workdir /workspace $image_tag
    else
        # To run a command through fish with all aliases available, we use fish -i -c
        set -l cmd_str (string escape -- $sandbox_args | string join " ")
        docker run -it --rm $mounts $env_vars --workdir /workspace $image_tag /usr/bin/fish -i -c "$cmd_str"
    end
end