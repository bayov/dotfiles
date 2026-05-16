local wezterm = require("wezterm")
local act = wezterm.action

local bayov_color_scheme = require("bayov_color_scheme")

-- Helpers specific to the copy mode key table.
local module = {}

-- Tint the background and dim inactive panes while in copy mode so it's
-- obvious the keyboard is captured. Only mutates overrides on transitions
-- to avoid spamming reloads on every status tick.
wezterm.on("update-status", function(window, _pane)
    local in_copy = window:active_key_table() == "copy_mode"
    local overrides = window:get_config_overrides() or {}
    local currently_tinted = overrides.colors
        and overrides.colors.background == bayov_color_scheme.copy_mode_bg
    if in_copy and not currently_tinted then
        overrides.colors = { background = bayov_color_scheme.copy_mode_bg }
        overrides.inactive_pane_hsb = { saturation = 0.7, brightness = 0.5 }
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

-- Builds a CopyMode action from a `motion`:
--   string  → act.CopyMode("Move" .. motion) (e.g. "Left", "ForwardWord")
--   table   → act.CopyMode(motion)            (e.g. { MoveByPage = -0.5 })
local function copy_mode_action(motion)
    if type(motion) == "string" then
        return act.CopyMode("Move" .. motion)
    end
    return act.CopyMode(motion)
end

-- Clears any active selection, then performs `motion`.
-- Mirror of ExtendSelection for unshifted movement keys.
module.Move = function(motion)
    return act.Multiple({
        act.ClearSelection,
        act.CopyMode("ClearSelectionMode"),
        copy_mode_action(motion),
    })
end

-- Extends the copy-mode selection by `motion`, starting a Cell selection
-- first if none is active.
module.ExtendSelection = function(motion)
    local action = copy_mode_action(motion)
    return wezterm.action_callback(function(window, pane)
        if window:get_selection_text_for_pane(pane) == "" then
            window:perform_action(
                act.CopyMode({ SetSelectionMode = "Cell" }),
                pane
            )
        end
        window:perform_action(action, pane)
    end)
end

return module
