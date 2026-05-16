local wezterm = require("wezterm")

-- This will hold the module we export.
local module = {}

-- Background tints applied while in copy/search mode. Subtly shifted
-- from the normal theme background (#1d1f21) so the mode is noticeable
-- without being jarring. Search mode is yellow-shifted to distinguish
-- it from copy mode's blue shift.
module.copy_mode_bg = "#1a1d26"
module.search_mode_bg = "#2e2a10"

function module.apply_to_config(config)
    -- Other themes to try (maybe)
    --     local color_scheme = wezterm.color.get_builtin_schemes()["Nucolors (terminal.sexy)"]
    --     local color_scheme = wezterm.color.get_builtin_schemes()["Peppermint (Gogh)"]
    --     local color_scheme = wezterm.color.get_builtin_schemes()["zenburn (terminal.sexy)"]
    --     local color_scheme = wezterm.color.get_builtin_schemes()["X::DotShare (terminal.sexy)"]
    --     local color_scheme = wezterm.color.get_builtin_schemes()["Ocean Dark (Gogh)"]
    local color_scheme =
        wezterm.color.get_builtin_schemes()["Tomorrow (dark) (terminal.sexy)"]

    -- change ANSI 8 (Bright Black / Gray):
    --
    -- Note:
    --  - ANSI 0-7 are in 'ansi'
    --  - ANSI 8-15 are in 'brights'
    color_scheme.brights[1] = "#666666"

    color_scheme.background = "#1d1f21"
    color_scheme.cursor_bg = "#ced0d6"
    color_scheme.cursor_fg = "black"
    color_scheme.cursor_border = "#ced0d6"
    color_scheme.scrollbar_thumb = "#454547"
    color_scheme.selection_fg = "white"
    color_scheme.selection_bg = "#214283"
    color_scheme.copy_mode_active_highlight_fg = { Color = "white" }
    color_scheme.copy_mode_active_highlight_bg = { Color = "#1a5463" }
    color_scheme.copy_mode_inactive_highlight_fg = { Color = "white" }
    color_scheme.copy_mode_inactive_highlight_bg = { Color = "#464948" }
    color_scheme.quick_select_label_bg = { Color = "#ea9e46" }
    color_scheme.quick_select_label_fg = { Color = "black" }
    color_scheme.quick_select_match_bg = { Color = "#fdff50" }
    color_scheme.quick_select_match_fg = { Color = "black" }
    config.color_schemes = { ["Bayov Scheme"] = color_scheme }
    config.color_scheme = "Bayov Scheme"

    local font_family = "JetBrains Mono NL"
    config.font_rules = {
        -- Faint text color same as bright-black (ANSI 8)
        {
            -- This rule applies to \x1b[2m (faint) text
            intensity = "Half",
            italic = false,
            font = wezterm.font(
                font_family,
                { foreground = "#666666", weight = "Regular" }
            ),
        },
        {
            -- This rule applies to \x1b[2;3m (faint+italic) text
            intensity = "Half",
            italic = true,
            font = wezterm.font(
                font_family,
                { foreground = "#666666", weight = "Regular", italic = true }
            ),
        },
        -- Bold actually bold
        {
            intensity = "Bold",
            font = wezterm.font(font_family, { weight = "ExtraBold" }),
        },
        {
            -- Bold and italic not working for me for some reason... Fix later.
            intensity = "Bold",
            italic = true,
            font = wezterm.font(
                font_family,
                { weight = "ExtraBold", italic = true }
            ),
        },
    }
end

-- Export the module
return module
