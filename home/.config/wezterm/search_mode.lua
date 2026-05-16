local wezterm = require("wezterm")

local bayov_color_scheme = require("bayov_color_scheme")

local module = {}

-- Tint the background and dim inactive panes while in search mode so it's
-- obvious the keyboard is captured. Uses a different tint than copy mode
-- to distinguish the two. Only mutates overrides on transitions to avoid
-- spamming reloads on every status tick.
wezterm.on("update-status", function(window, _pane)
    local in_search = window:active_key_table() == "search_mode"
    local overrides = window:get_config_overrides() or {}
    local currently_tinted = overrides.colors
        and overrides.colors.background == bayov_color_scheme.search_mode_bg
    if in_search and not currently_tinted then
        overrides.colors = { background = bayov_color_scheme.search_mode_bg }
        overrides.inactive_pane_hsb = { saturation = 0.7, brightness = 0.5 }
        window:set_config_overrides(overrides)
    elseif not in_search and currently_tinted then
        overrides.colors = nil
        overrides.inactive_pane_hsb = nil
        window:set_config_overrides(overrides)
    end
end)

return module
