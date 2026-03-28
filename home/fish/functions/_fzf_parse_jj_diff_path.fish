# Used as a helper for `_fzf_search_jj_status``
function _fzf_parse_jj_diff_path --description "Extract the path from a jj diff --summary line, handling renames."
    string sub --start 3 $argv | string replace -r '\{(.*) => (.*)\}' '$2'
end
