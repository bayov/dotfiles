function cleanup_empty_workspaces --description 'Cleans up empty Fig and Piper workspaces, with a dry-run option and colorized output.'
    # Parse arguments
    argparse 'h/help' 'dry-run' -- $argv
    
    if set -q _flag_help
        echo "Usage: cleanup_empty_fig_workspaces [--dry-run]"
        echo "  --dry-run : Print actions without executing them."
        return 0
    end
    
    set -l is_dry_run false
    if set -q _flag_dry_run
        set is_dry_run true
        echo (set_color magenta)"--- DRY RUN MODE ENABLED ---"(set_color normal)
        echo (set_color magenta)"No actual deletions or automated cleanups will be performed."(set_color normal)
        echo (set_color magenta)"----------------------------"(set_color normal)
    end
    
    set -l ORIGINAL_DIR (pwd)
    set -l FIG_CLIENTS
    set -l client_name
    set -l client_check_status
    
    # --- Fig Client Cleanup ---
    echo ""
    echo (set_color cyan)"--- Starting Fig Client Cleanup ---"(set_color normal)
    cd "$ORIGINAL_DIR"
    set FIG_CLIENTS (hg citc --list)
    
    if test (count $FIG_CLIENTS) -eq 0
        echo (set_color yellow)"No Fig clients found to process."(set_color normal)
    else
        echo (set_color cyan)"Found Fig clients:"(set_color normal)
        for item in $FIG_CLIENTS
            echo "- "(set_color blue)"$item"(set_color normal)
        end
        
        function _cleanup_check_client_emptiness_internal --description "Checks if a Fig client is empty. Do not call directly."
            set -l client_to_check $argv[1]
            set -l parent_original_dir $argv[2]
            set -l in_client_dir false
            
            if hgd "$client_to_check"
                set in_client_dir true
                echo "  Currently in: "(set_color blue)"$(pwd)"(set_color normal)
            else
                echo "  "(set_color red)"ERROR: Could not navigate to Fig client '$client_to_check' using hgd. Skipping."(set_color normal)
                return 1
            end
            
            set -l UNCOMMITTED_CHANGES (hg status -mardu)
            if test (count $UNCOMMITTED_CHANGES) -gt 0
                echo "  "(set_color yellow)"INFO: Fig client '$client_to_check' has uncommitted changes. Skipping deletion."(set_color normal)
                echo "  Uncommitted files found:"
                for line in $UNCOMMITTED_CHANGES; echo "    $line"; end
                if $in_client_dir; cd "$parent_original_dir"; end
                return 1
            end
            
            set -l ACTIVE_DRAFT_COMMITS (hg log -r "head() and draft() and not obsolete()" --template "x")
            if test (count $ACTIVE_DRAFT_COMMITS) -gt 0
                echo "  "(set_color yellow)"INFO: Fig client '$client_to_check' has active draft/unuploaded commits. Skipping deletion."(set_color normal)
                if $in_client_dir; cd "$parent_original_dir"; end
                return 1
            end
            
            set -l PENDING_CLS_OUTPUT (hg cls)
            set -l pending_cls_content (string join "" $PENDING_CLS_OUTPUT)
            if string length -q -- "$pending_cls_content"
                echo "  "(set_color yellow)"INFO: Fig client '$client_to_check' has pending CLs. Skipping deletion."(set_color normal)
                echo "  'hg cls' output:"
                for line in $PENDING_CLS_OUTPUT; echo "    $line"; end
                if $in_client_dir; cd "$parent_original_dir"; end
                return 1
            end
            
            echo "  "(set_color green)"INFO: Fig client '$client_to_check' appears to be empty."(set_color normal)
            if $in_client_dir; cd "$parent_original_dir"; end
            return 0
        end
        
        for client_name in $FIG_CLIENTS
            echo (set_color cyan)"Processing Fig client: "(set_color blue)"$client_name"(set_color normal)
            cd "$ORIGINAL_DIR"
            
            _cleanup_check_client_emptiness_internal "$client_name" "$ORIGINAL_DIR"
            set client_check_status $status
            
            cd "$ORIGINAL_DIR"
            
            if test $client_check_status -eq 0
                if $is_dry_run
                    echo "  "(set_color magenta)"DRY RUN: Would execute: hg citc -d '$client_name'"(set_color normal)
                else
                    echo "  "(set_color yellow)"Attempting to delete Fig client '$client_name'..."(set_color normal)
                    hg citc -d "$client_name"
                    if test $status -eq 0
                        echo "  "(set_color green)"SUCCESS: Fig client '$client_name' deleted."(set_color normal)
                    else
                        echo "  "(set_color red)"ERROR: Failed to delete Fig client '$client_name'."(set_color normal)
                    end
                end
            else
                echo "  "(set_color yellow)"Skipped deletion for Fig client '$client_name' based on checks."(set_color normal)
            end
        end
    end
    echo (set_color cyan)"--- Finished Fig Client Cleanup ---"(set_color normal)
    echo ""
    
    
    # --- Piper Client Cleanup ---
    echo (set_color cyan)"--- Starting Piper Client Cleanup ---"(set_color normal)
    cd "$ORIGINAL_DIR" # Ensure we are in a known directory
    
    if $is_dry_run
        echo (set_color magenta)"DRY RUN: Would execute: g4 cleanup --delete_empty"(set_color normal)
    else
        echo (set_color yellow)"Attempting to automatically cleanup empty Piper clients using 'g4 cleanup --delete_empty'..."(set_color normal)
        echo (set_color yellow)"This will non-interactively delete Piper clients with no open files, pending changes, or nothave files."(set_color normal)
        g4 cleanup --delete_empty
        if test $status -eq 0
            echo (set_color green)"'g4 cleanup --delete_empty' completed."(set_color normal)
        else
            echo (set_color red)"'g4 cleanup --delete_empty' finished with status: $status"(set_color normal)
        end
    end
    echo (set_color cyan)"--- Finished Piper Client Cleanup ---"(set_color normal)
    echo ""
    
    
    # --- Final Message ---
    cd "$ORIGINAL_DIR"
    echo (set_color green)"Workspace cleanup attempt finished."(set_color normal)
end
