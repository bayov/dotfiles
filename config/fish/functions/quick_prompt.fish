function quick_prompt --description "Answers a super quick agent prompt using a free & fast LLM"
    set -lx OPENCODE_DISABLE_DEFAULT_PLUGINS true
    set -lx OPENCODE_DISABLE_CLAUDE_CODE true
    set -lx OPENCODE_MODEL "opencode/minimax-m2.5-free"
    
    set -l res_json (echo "$argv" | opencode run --model "$OPENCODE_MODEL" --format json)
    set -l res_text (echo $res_json | jq -r 'select(.type == "text") | .part.text')

    string collect -- $res_text
end
