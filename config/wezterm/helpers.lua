local wezterm = require("wezterm")
local act = wezterm.action

-- This will hold the module we export.
local module = {}

-- Copies selected text in the active pane to clipboard, or sends Ctrl+C
-- (SIGINT) to the terminal if no text is selected.
module.CopyOrInterrupt = wezterm.action_callback(function (window, pane)
    selection_text = window:get_selection_text_for_pane(pane)
    is_selection_active = string.len(selection_text) ~= 0
    if is_selection_active then
        window:perform_action(act.CopyTo("Clipboard"), pane)
        window:perform_action(act.ClearSelection, pane)
    else
        window:perform_action(act.SendKey{ mods="CTRL", key="c" }, pane)
    end
end)

module.CloseAndClearSelection = act.Multiple({
    act.CopyMode("Close"),
    act.ClearSelection,
})

-- Export the module
return module
