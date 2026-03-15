function test_personal_project
    set -l jj_root (jj root 2>/dev/null)
    test -n "$jj_root" || return 0

    set -l marker "$jj_root/.bayov-personal-project"
    set -l test_script "$jj_root/dev/test.py"

    # Run if marker and script exist
    if test -f "$marker"; and test -f "$test_script"
        echo -e "\e[35mTesting...\e[0m"
        uv run "$test_script" --quiet
        set -l result $status
        echo
        return $result
    end

    return 0
end
