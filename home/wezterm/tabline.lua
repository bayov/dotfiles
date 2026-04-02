local wezterm = require("wezterm")
local zoom = require("zoom")

-- This will hold the module we export.
local module = {}

local function active_key_table(window)
    local name = window:active_key_table()
    if name == "pane" then
        return " key:" .. name .. " "
    end
    return " "
end

module.plugin =
    wezterm.plugin.require("https://github.com/michaelbrusegard/tabline.wez")
module.plugin.setup({
    options = {
        icons_enabled = true,
        tabs_enabled = true,
        theme = "Tomorrow (dark) (terminal.sexy)",
        theme_overrides = {
            --         normal_mode = {
            --             a = { fg = "white", bg = "#2b2d30" },
            --             b = { fg = "white", bg = "#2b2d30" },
            --         },
            --         copy_mode = {
            --             b = { fg = "white", bg = "#2b2d30" },
            --             z = { fg = "white", bg = "#2b2d30" },
            --         },
            --         search_mode = {
            --             b = { fg = "white", bg = "#2b2d30" },
            --             z = { fg = "white", bg = "#2b2d30" },
            --         },
            --         tab = {
            --             active = { fg = "white", bg = "#2b2d30" },
            --             inactive = { fg = "grey", bg = "#1e1f22" },
            --             inactive_hover = { fg = "white", bg = "#1e1f22" },
            --         },
        },
        section_separators = {
            left = "",
            right = "",
        },
        component_separators = {
            left = "",
            right = "",
        },
        tab_separators = {
            left = "",
            right = " ",
        },
    },
    sections = {
        tabline_a = { "mode" },
        tabline_b = {
            -- "workspace",
        },
        tabline_c = { "" },
        tab_active = {
            { "tab", padding = { left = 2, right = 1 } },
            "(",
            { "cwd", padding = 0 },
            ")",
            { "zoomed", padding = { left = 1, right = 0 } },
        },
        tab_inactive = {
            { "tab", padding = { left = 2, right = 1 } },
            "(",
            { "cwd", padding = 0 },
            ")",
            { "zoomed", padding = { left = 1, right = 0 } },
        },
        tabline_x = {
            active_key_table,
            zoom.tabline_component,
            -- "ram",
        },
        tabline_y = {
            -- "datetime",
            -- "battery",
        },
        tabline_z = {
            -- "domain",
        },
    },
    extensions = {},
})

-- Export the module
return module
