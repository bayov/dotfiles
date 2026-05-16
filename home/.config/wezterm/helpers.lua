local wezterm = require("wezterm")
local act = wezterm.action

-- This will hold the module we export.
local module = {}

-- Copies selected text in the active pane to clipboard, or sends Ctrl+C
-- (SIGINT) to the terminal if no text is selected.
module.CopyOrInterrupt = wezterm.action_callback(function(window, pane)
    selection_text = window:get_selection_text_for_pane(pane)
    is_selection_active = string.len(selection_text) ~= 0
    if is_selection_active then
        window:perform_action(act.CopyTo("Clipboard"), pane)
        window:perform_action(act.ClearSelection, pane)
    else
        window:perform_action(act.SendKey({ mods = "CTRL", key = "c" }), pane)
    end
end)

module.CloseAndClearSelection = act.Multiple({
    act.CopyMode("Close"),
    act.ClearSelection,
})

-- Extends the copy-mode selection in `direction` ("Left"/"Right"/"Up"/"Down"),
-- starting a Cell selection first if none is active.
module.ExtendSelection = function(direction)
    return wezterm.action_callback(function(window, pane)
        if window:get_selection_text_for_pane(pane) == "" then
            window:perform_action(
                act.CopyMode({ SetSelectionMode = "Cell" }),
                pane
            )
        end
        window:perform_action(act.CopyMode("Move" .. direction), pane)
    end)
end

-- Clears the current pane's viewport while preserving its content in
-- scrollback buffer. This is useful when you want to clear the screen
-- but keep the ability to scroll up to see previous output.
module.ClearPaneWhileKeepingScrollback = wezterm.action_callback(
    function(window, pane)
        -- avoid running in TUI programs
        if pane:is_alt_screen_active() then
            window:perform_action(
                act.SendKey({ mods = "ALT", key = "l" }),
                pane
            )
            return
        end

        -- scroll to bottom
        window:perform_action(act.ScrollToBottom, pane)

        -- get the current height of the viewport
        local height = pane:get_dimensions().viewport_rows

        -- inject a string of new lines equal to the viewport height
        -- local blank_viewport = string.rep("\r\n", height)
        -- pane:inject_output(blank_viewport)

        -- send an escape sequence to clear the viewport (CTRL-L)
        pane:send_text("\x0c")
    end
)

-- Wraps a WezTerm `key_bind` (provided as `{mods, key, action}`) and returns
-- a new keybind where `action` is performed only when the current pane is not
-- a TUI. When the pane is a TUI, sends the key as is instead.
module.KeyUnlessTui = function(key_bind)
    local mods = key_bind.mods
    local key = key_bind.key
    local action = key_bind.action

    return {
        mods = mods,
        key = key,
        action = wezterm.action_callback(function(window, pane)
            if not pane:is_alt_screen_active() then
                window:perform_action(action, pane)
            else
                window:perform_action(
                    act.SendKey({ key = key, mods = mods }),
                    pane
                )
            end
        end),
    }
end

-- Export the module
return module
