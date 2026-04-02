local wezterm = require("wezterm")
local act = wezterm.action

-- This will hold the module we export.
local module = {}

-- Base font size (should match config.font_size in wezterm.lua)
--
-- Set `wezterm.GLOBAL.base_font_size = config.font_size` in wezterm.lua to
-- ensure the base font size is capture correctly.
local function get_base_font_size()
    return wezterm.GLOBAL.base_font_size or 11
end

-- Returns the current zoom level for the given `window`.
function module.get_zoom_level(window)
    if not window then
        return 1.0
    end
    local overrides = window:get_config_overrides() or {}
    if overrides.font_size then
        return overrides.font_size / get_base_font_size()
    end
    return 1.0
end

-- Sets the zoom level on the given `window`.
--
-- Emits the following events:
--  * `bayov.zoom.changed`.
--  * `bayov.zoom.active` when `zoom_level` is set to anything other than 1.0.
--  * `bayov.zoom.inactive` when `zoom_level` is set to 1.0.
function module.set_zoom_level(zoom_level, window, pane)
    if not window then
        wezterm.log_error("zoom.set_zoom_level: window is nil")
        return
    end

    zoom_level = math.floor(zoom_level * 10 + 0.5) / 10 -- Round to 1 decimal place
    zoom_level = math.max(0.1, zoom_level)

    local base_font_size = get_base_font_size()
    local overrides = window:get_config_overrides() or {}

    if math.abs(zoom_level - 1.0) < 0.01 then
        overrides.font_size = nil
        wezterm.log_info(
            string.format(
                "zoom.set_zoom_level: resetting to base font_size (%d)",
                base_font_size
            )
        )
    else
        overrides.font_size = base_font_size * zoom_level
        wezterm.log_info(
            string.format(
                "zoom.set_zoom_level: setting font_size to %.1f",
                overrides.font_size
            )
        )
    end
    window:set_config_overrides(overrides)

    wezterm.emit("bayov.zoom.changed", window, pane)
    if math.abs(zoom_level - 1.0) < 0.01 then
        wezterm.emit("bayov.zoom.inactive", window, pane)
    else
        wezterm.emit("bayov.zoom.active", window, pane)
    end
end

function module.tabline_component(window, pane)
    local zoom_level = module.get_zoom_level(window)
    if math.abs(zoom_level - 1.0) < 0.01 then
        return ""
    end
    return string.format(" Zoom: %.1fx ", zoom_level)
end

module.ZoomIn = wezterm.action_callback(function(window, pane)
    module.set_zoom_level(module.get_zoom_level(window) + 0.1, window, pane)
end)

module.ZoomOut = wezterm.action_callback(function(window, pane)
    module.set_zoom_level(module.get_zoom_level(window) - 0.1, window, pane)
end)

module.ResetZoom = wezterm.action_callback(function(window, pane)
    module.set_zoom_level(1.0, window, pane)
end)

-- Export the module
return module
