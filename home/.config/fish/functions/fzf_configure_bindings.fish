# Always installs bindings for insert and default mode for simplicity and b/c it has almost no side-effect
# https://gitter.im/fish-shell/fish-shell?at=60a55915ee77a74d685fa6b1
function fzf_configure_bindings --description "Installs the default key bindings for fzf.fish with user overrides passed as options."
    # no need to install bindings if not in interactive mode or running tests
    status is-interactive || test "$CI" = true; or return

    set -f options_spec h/help 'directory=?' 'jj_log=?' 'jj_status=?' 'history=?' 'processes=?' 'variables=?' 'projects=?' 'zellij_paths=?' 'zellij_urls=?'
    argparse --max-args=0 --ignore-unknown $options_spec -- $argv 2>/dev/null
    if test $status -ne 0
        echo "Invalid option or a positional argument was provided." >&2
        _fzf_configure_bindings_help
        return 22
    else if set --query _flag_help
        _fzf_configure_bindings_help
        return
    else
        # Initialize with default key sequences and then override or disable them based on flags
        # index 1 = directory, 2 = jj_log, 3 = jj_status, 4 = history, 5 = processes, 6 = variables, 7 = projects, 8 = zellij_paths, 9 = zellij_urls
        set -f key_sequences \ef \e2 \ed \cr \e\cp \e\ce \ew \eF \eu \en # \c = control, \e = alt
        set --query _flag_directory && set key_sequences[1] "$_flag_directory"
        set --query _flag_jj_log && set key_sequences[2] "$_flag_jj_log"
        set --query _flag_jj_status && set key_sequences[3] "$_flag_jj_status"
        set --query _flag_history && set key_sequences[4] "$_flag_history"
        set --query _flag_processes && set key_sequences[5] "$_flag_processes"
        set --query _flag_variables && set key_sequences[6] "$_flag_variables"
        set --query _flag_projects && set key_sequences[7] "$_flag_projects"
        set --query _flag_zellij_paths && set key_sequences[8] "$_flag_zellij_paths"
        set --query _flag_zellij_urls && set key_sequences[9] "$_flag_zellij_urls"

        # If fzf bindings already exists, uninstall it first for a clean slate
        if functions --query _fzf_uninstall_bindings
            _fzf_uninstall_bindings
        end

        for mode in default insert
            test -n "$key_sequences[1]" && bind --mode $mode $key_sequences[1] _fzf_search_directory
            test -n "$key_sequences[2]" && bind --mode $mode $key_sequences[2] _fzf_search_jj_log
            test -n "$key_sequences[3]" && bind --mode $mode $key_sequences[3] _fzf_search_jj_status
            test -n "$key_sequences[4]" && bind --mode $mode $key_sequences[4] _fzf_search_history
            test -n "$key_sequences[5]" && bind --mode $mode $key_sequences[5] _fzf_search_processes
            test -n "$key_sequences[6]" && bind --mode $mode $key_sequences[6] "$_fzf_search_vars_command"
            test -n "$key_sequences[7]" && bind --mode $mode $key_sequences[7] _fzf_search_projects
            test -n "$key_sequences[8]" && bind --mode $mode $key_sequences[8] _fzf_search_zellij_paths
            test -n "$key_sequences[9]" && bind --mode $mode $key_sequences[9] _fzf_search_zellij_urls
        end

        function _fzf_uninstall_bindings --inherit-variable key_sequences
            bind --erase -- $key_sequences
            bind --erase --mode insert -- $key_sequences
        end
    end
end
