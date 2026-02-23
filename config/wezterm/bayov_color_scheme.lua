local wezterm = require("wezterm")

-- This will hold the module we export.
local module = {}

function module.apply_to_config(config)
    -- Other themes to try (maybe)
--     local color_scheme = wezterm.color.get_builtin_schemes()["Nucolors (terminal.sexy)"]
--     local color_scheme = wezterm.color.get_builtin_schemes()["Peppermint (Gogh)"]
--     local color_scheme = wezterm.color.get_builtin_schemes()["zenburn (terminal.sexy)"]
--     local color_scheme = wezterm.color.get_builtin_schemes()["X::DotShare (terminal.sexy)"]
--     local color_scheme = wezterm.color.get_builtin_schemes()["Ocean Dark (Gogh)"]
    local color_scheme = wezterm.color.get_builtin_schemes()["Tomorrow (dark) (terminal.sexy)"]
    --color_scheme.background = "#1f2023"
    color_scheme.cursor_bg = "#ced0d6"
    color_scheme.cursor_fg = "black"
    color_scheme.cursor_border = "#ced0d6"
    color_scheme.scrollbar_thumb = "#454547"
    color_scheme.selection_fg = "white"
    color_scheme.selection_bg = "#214283"
    color_scheme.copy_mode_active_highlight_fg = {Color = "white"}
    color_scheme.copy_mode_active_highlight_bg = {Color = "#1a5463"}
    color_scheme.copy_mode_inactive_highlight_fg = {Color = "white"}
    color_scheme.copy_mode_inactive_highlight_bg = {Color = "#464948"}
    color_scheme.quick_select_label_bg = {Color = "#ea9e46"}
    color_scheme.quick_select_label_fg = {Color = "black"}
    color_scheme.quick_select_match_bg = {Color = "#fdff50"}
    color_scheme.quick_select_match_fg = {Color = "black"}
    config.color_schemes = {["Bayov Scheme"] = color_scheme}
    config.color_scheme = "Bayov Scheme"
end

-- Export the module
return module
