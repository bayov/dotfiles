function format_personal_project
    if test "$use_google_config" = "true"
        jj fix
	return
    end
    set -l jj_root (jj root 2>/dev/null)
    test -n "$jj_root" || return

    set -l marker "$jj_root/.bayov-personal-project"
    set -l format_script "$jj_root/dev/format.py"

    # Run if marker and script exist
    if test -f "$marker"; and test -f "$format_script"
        echo -e "\e[35mFormatting...\e[0m"
        uv run "$format_script" --all-tracked
        return $status
    end

    return 0
end
