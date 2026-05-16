local wezterm = require("wezterm")
local act = wezterm.action

local bayov_color_scheme = require("bayov_color_scheme")

-- Helpers specific to copy mode / search mode key tables.
local module = {}

-- Tint the background and dim inactive panes while in copy/search mode so
-- it's obvious the keyboard is captured. Only mutates overrides on
-- transitions to avoid spamming reloads on every status tick.
wezterm.on("update-status", function(window, _pane)
    local key_table = window:active_key_table()
    local in_copy = key_table == "copy_mode" or key_table == "search_mode"
    local overrides = window:get_config_overrides() or {}
    local currently_tinted = overrides.colors
        and overrides.colors.background == bayov_color_scheme.copy_mode_bg
    if in_copy and not currently_tinted then
        overrides.colors = { background = bayov_color_scheme.copy_mode_bg }
        overrides.inactive_pane_hsb = { saturation = 0.6, brightness = 0.5 }
        window:set_config_overrides(overrides)
    elseif not in_copy and currently_tinted then
        overrides.colors = nil
        overrides.inactive_pane_hsb = nil
        window:set_config_overrides(overrides)
    end
end)

module.CloseAndClearSelection = act.Multiple({
    act.CopyMode("Close"),
    act.ClearSelection,
})

-- If a selection is active, clear it but stay in copy mode.
-- Otherwise, close copy mode.
module.ClearSelectionOrCloseCopyMode = wezterm.action_callback(
    function(window, pane)
        if window:get_selection_text_for_pane(pane) ~= "" then
            window:perform_action(
                act.Multiple({
                    act.ClearSelection,
                    act.CopyMode("ClearSelectionMode"),
                }),
                pane
            )
        else
            window:perform_action(module.CloseAndClearSelection, pane)
        end
    end
)

-- Extends the copy-mode selection in `direction` ("Left"/"Right"/"Up"/"Down"
-- / "ToStartOfLineContent" / "ToEndOfLineContent" / ...), starting a Cell
-- selection first if none is active.
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

return module
