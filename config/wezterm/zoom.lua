local wezterm = require("wezterm")
local act = wezterm.action

-- This will hold the module we export.
local module = {}

-- Initialize a global variable to store the current zoom level
if wezterm.GLOBAL.zoom_level == nil then
  wezterm.GLOBAL.zoom_level = 1.0
end

-- Returns the current zoom level.
function module.get_zoom_level()
    return wezterm.GLOBAL.zoom_level
end

-- Sets the zoom level on the given `pane`.
--
-- Emits the following events (only if the zoom level has changed:
--  * `bayov.zoom.changed`.
--  * `bayov.zoom.active` when `zoom_level` is set to anything other than 1.0.
--  * `bayov.zoom.inactive` when `zoom_level` is set to 1.0.
function module.set_zoom_level(zoom_level, window, pane)
    local prev_zoom_level = wezterm.GLOBAL.zoom_level
    wezterm.GLOBAL.zoom_level = zoom_level
    if prev_zoom_level ~= zoom_level then
        wezterm.emit("bayov.zoom.changed", window, pane)
        if zoom_level == 1.0 then
            wezterm.emit("bayov.zoom.inactive", window, pane)
        else
            wezterm.emit("bayov.zoom.active", window, pane)
        end
    end
    window:perform_action(act.IncreaseFontSize, pane)
end

-- A tabline.wez component to show the current zoom level in the tab bar.
-- module.tabline_component = {
--  default_opts = {
--    icon = wezterm.nerdfonts.cod_server,
--  },
--  update = function(_, opts)
--      return "aaa"
--  end,
-- }
-- function module.tabline_component()
--     local zoom_level = wezterm.GLOBAL.zoom_level
-- --     if zoom_level == 1.0 then
-- --         return ""
-- --     end
--     return {
--         text = string.format("Zoom: %.1fx", wezterm.GLOBAL.zoom_level)
--     }
-- end
--
module.ZoomIn = wezterm.action_callback(function(window, pane)
    module.set_zoom_level(
        (wezterm.GLOBAL.zoom_level or 1.0) + 0.1,
        window,
        pane
    )
end)

module.ZoomOut = wezterm.action_callback(function(window, pane)
    module.set_zoom_level(
        math.max((wezterm.GLOBAL.zoom_level or 1.0) - 0.1, 0.1),
        window,
        pane
    )
end)

module.ResetZoom = wezterm.action_callback(function(window, pane)
    module.set_zoom_level(1.0, window, pane)
end)

-- Export the module
return module
