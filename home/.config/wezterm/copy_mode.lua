local wezterm = require("wezterm")
local act = wezterm.action

local bayov_color_scheme = require("bayov_color_scheme")
local helpers = require("helpers")

-- Helpers specific to the copy mode key table.
local module = {}

helpers.TintWhileKeyTableActive("copy_mode", bayov_color_scheme.copy_mode_bg)

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
