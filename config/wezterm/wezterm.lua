local wezterm = require("wezterm")
local act = wezterm.action

local helpers = require("helpers")
local bayov_color_scheme = require("bayov_color_scheme")
local tabline = require("tabline")
local zoom = require("zoom")

-- This will hold the configuration we export.
local config = wezterm.config_builder()

--------------------------------------------------------------------------------
-- Scrollback
--------------------------------------------------------------------------------
config.scrollback_lines = 50000
config.enable_scroll_bar = false

--------------------------------------------------------------------------------
-- Theme & Appearance
--------------------------------------------------------------------------------
-- Font
config.font_size = 10
wezterm.GLOBAL.base_font_size = config.font_size
config.harfbuzz_features = {"calt=0", "clig=0", "liga=0"} -- disable ligatures
-- Colors
bayov_color_scheme.apply_to_config(config)
config.inactive_pane_hsb = {
    saturation = 0.9,
    brightness = 0.8,
}
-- Tabs
tabline.plugin.apply_to_config(config)
config.use_fancy_tab_bar = false
config.tab_bar_at_bottom = true
config.window_decorations = "TITLE|RESIZE"
-- Window
config.initial_cols = 120
config.initial_rows = 28
config.window_padding = {
    left = "1cell",
    right = "1cell",
    top = "0.5cell",
    bottom = "0",
}
-- Cursor
config.cursor_blink_rate = 0
config.default_cursor_style = 'SteadyBlock'
-- Misc
config.max_fps = 120
config.disable_default_quick_select_patterns = true
config.quick_select_patterns = {
    "[0-9a-f]{7,40}",
    -- Jujustu change IDs
    --
    -- Note:
    -- Technically, WezTerm docs say that capture groups should not be used as
    -- the patterns above are compiled into a single regex and these groups
    -- might interfere. However, I found that the above just works for now.
    "[@◆○×│] {2,}([a-z]{8})",
}
config.quick_select_alphabet = "qwerasdzxcv"

-- Start in full-screen
wezterm.on("gui-startup", function (window)
    local tab, pane, window = wezterm.mux.spawn_window(cmd or {})
    local gui_window = window:gui_window();
    gui_window:perform_action(act.ToggleFullScreen, pane)
end)

--------------------------------------------------------------------------------
-- Keyboard bindings
--------------------------------------------------------------------------------
config.disable_default_key_bindings = true
config.keys = {
    -- Copy & Paste
    {mods = "CTRL", key = "c", action = helpers.CopyOrInterrupt},
    {mods = "CTRL|SHIFT", key = "c", action = act.CopyTo("Clipboard")},
    {mods = "CTRL", key = "v", action = act.PasteFrom("Clipboard")},
    {mods = "CTRL|SHIFT", key = "v", action = act.PasteFrom("Clipboard")},

    -- Scrolling & Scrollback
    helpers.KeyUnlessTui({mods = "NONE", key = "PageUp", action = act.ScrollByPage(-0.5)}),
    helpers.KeyUnlessTui({mods = "NONE", key = "PageDown", action = act.ScrollByPage(0.5)}),
    helpers.KeyUnlessTui({mods = "ALT", key = "UpArrow", action = act.ScrollByLine(-2)}),
    helpers.KeyUnlessTui({mods = "ALT", key = "DownArrow", action = act.ScrollByLine(2)}),
    helpers.KeyUnlessTui({mods = "CTRL|ALT", key = "UpArrow", action = act.ScrollToPrompt(-1)}),
    helpers.KeyUnlessTui({mods = "CTRL|ALT", key = "DownArrow", action = act.ScrollToPrompt(1)}),
    helpers.KeyUnlessTui({mods = "ALT", key = "Home", action = act.ScrollToTop}),
    helpers.KeyUnlessTui({mods = "ALT", key = "End", action = act.ScrollToBottom}),
    helpers.KeyUnlessTui({mods = "ALT", key = "l", action = helpers.ClearPaneWhileKeepingScrollback}),
    helpers.KeyUnlessTui({
        mods = "CTRL|ALT",
        key = "l",
        action = act.Multiple({
            act.ClearScrollback("ScrollbackAndViewport"),
            act.SendKey({mods="CTRL", key="L"}),
        }),
    }),

    -- Search/QuickSelect/Copy modes
    {
        mods = "CTRL|SHIFT",
        key = "f",
        action = wezterm.action_callback(function(window, pane)
            window:perform_action(act.Search({ CaseInSensitiveString = "" }), pane)
            window:perform_action(act.Search("CurrentSelectionOrEmptyString"), pane)
            window:perform_action(
                act.Multiple({
                    act.CopyMode("ClearPattern"),
                    act.CopyMode("ClearSelectionMode"),
                    act.CopyMode("MoveToScrollbackBottom"),
                }),
                pane
            )
        end)
    },
    {mods = "ALT", key = "v", action = act.QuickSelect},
    {mods = "ALT", key = "c", action = act.ActivateCopyMode},

    -- Presentation
    {mods = "ALT", key = "\\", action = act.ToggleFullScreen},
    {mods = "ALT", key = "-", action = zoom.ZoomOut},
    {mods = "ALT", key = "=", action = zoom.ZoomIn},
    {mods = "ALT", key = "0", action = zoom.ResetZoom},

    -- Tab-management
    {mods = "ALT", key = "t", action = act.SpawnTab("DefaultDomain")},
    {mods = "ALT", key = "LeftArrow", action = act.ActivateTabRelative(-1)},
    {mods = "ALT", key = "RightArrow", action = act.ActivateTabRelative(1)},
    {mods = "ALT", key = "[", action = act.MoveTabRelative(-1)},
    {mods = "ALT", key = "]", action = act.MoveTabRelative(1)},

    -- Pane-management
    {mods = "ALT", key = "s", action = act.ActivateKeyTable({name = "pane"})},
    {mods = "ALT|SHIFT", key = "UpArrow", action = act.ActivatePaneDirection("Up")},
    {mods = "ALT|SHIFT", key = "DownArrow", action = act.ActivatePaneDirection("Down")},
    {mods = "ALT|SHIFT", key = "LeftArrow", action = act.ActivatePaneDirection("Left")},
    {mods = "ALT|SHIFT", key = "RightArrow", action = act.ActivatePaneDirection("Right")},
    {mods = "CTRL|ALT", key = "Numpad8", action = act.AdjustPaneSize({"Up", 4})},
    {mods = "CTRL|ALT", key = "Numpad2", action = act.AdjustPaneSize({"Down", 4})},
    {mods = "CTRL|ALT", key = "Numpad4", action = act.AdjustPaneSize({"Left", 4})},
    {mods = "CTRL|ALT", key = "Numpad6", action = act.AdjustPaneSize({"Right", 4})},
    {mods = "ALT", key = "x", action = act.CloseCurrentPane({confirm = false})},

    -- Misc
    {mods = "ALT", key = "a", action = act.ActivateCommandPalette},
    {mods = "CTRL|ALT", key = "r", action = act.ReloadConfiguration},
    {mods = "ALT", key = "`", action = act.ShowDebugOverlay},

    -- Support Shift+Enter for CLI tools
    {mods = "SHIFT", key = "Enter", action = act.SendString("\x1b[13;2u")},

    -- Open URLs
    {
        mods = "ALT",
        key = "o",
        action = act.QuickSelectArgs({
            label = "open url",
            patterns = {
              "(https?://\\S+)¶",
            },
            action = wezterm.action_callback(function(window, pane)
              local url = window:get_selection_text_for_pane(pane)
              wezterm.log_info('opening: ' .. url)
              wezterm.open_with(url)
            end),
        }),
    },
}
--------------------------------------------------------------------------------
-- Keyboard bindings for specific modes (key tables)
--------------------------------------------------------------------------------
config.key_tables = {}
config.key_tables.copy_mode = {
    -- Close
    {mods = "NONE", key = "Escape", action = helpers.CloseAndClearSelection},
    {mods = "ALT", key = "c", action = helpers.CloseAndClearSelection},
    -- Perform copy (and then close)
    {
        mods = "CTRL",
        key = "c",
        action = act.Multiple({
            act.CopyTo("Clipboard"),
            act.ClearSelection,
            act.CopyMode("Close"),
        }),
    },
    {
        mods = "NONE",
        key = "y",
        action = act.Multiple({
            act.CopyTo("Clipboard"),
            act.ClearSelection,
            act.CopyMode("Close"),
        }),
    },
    -- Enter selection mode
    {mods = "NONE", key = "v", action = act.CopyMode({SetSelectionMode = "Cell"})},
    {mods = "SHIFT", key = "v", action = act.CopyMode({SetSelectionMode = "Line"})},
    {mods = "CTRL", key = "v", action = act.CopyMode({SetSelectionMode = "SemanticZone"})},
    {mods = "CTRL", key = "w", action = act.CopyMode({SetSelectionMode = "Word"})},
    {mods = "SHIFT", key = "Insert", action = act.CopyMode({SetSelectionMode = "Block"})},
    -- Movement
    {mods = "NONE", key = "UpArrow", action = act.CopyMode("MoveUp")},
    {mods = "NONE", key = "DownArrow", action = act.CopyMode("MoveDown")},
    {mods = "NONE", key = "LeftArrow", action = act.CopyMode("MoveLeft")},
    {mods = "NONE", key = "RightArrow", action = act.CopyMode("MoveRight")},
    {mods = "CTRL", key = "LeftArrow", action = act.CopyMode("MoveBackwardWord")},
    {mods = "CTRL", key = "RightArrow", action = act.CopyMode("MoveForwardWord")},
    {mods = "NONE", key = "Home", action = act.CopyMode("MoveToStartOfLineContent")},
    {mods = "NONE", key = "End", action = act.CopyMode("MoveToEndOfLineContent")},
    {mods = "CTRL", key = "Home", action = act.CopyMode("MoveToScrollbackTop")},
    {mods = "CTRL", key = "End", action = act.CopyMode("MoveToScrollbackBottom")},
    {mods = "NONE", key = "PageUp", action = act.CopyMode({MoveByPage = -0.5})},
    {mods = "NONE", key = "PageDown", action = act.CopyMode({MoveByPage = 0.5})},
    {mods = "SHIFT", key = "UpArrow", action = act.CopyMode("MoveBackwardSemanticZone")},
    {mods = "SHIFT", key = "DownArrow", action = act.CopyMode("MoveForwardSemanticZone")},
    -- Clear search and selection
    {
        mods = "ALT",
        key = "Delete",
        action = act.Multiple({
            act.CopyMode("ClearPattern"),
            act.CopyMode("ClearSelectionMode"),
            act.ClearSelection,
        }),
    },
}
config.key_tables.search_mode = {
    {mods = "NONE", key = "Escape", action = helpers.CloseAndClearSelection},
    {mods = "CTRL|SHIFT", key = "f", action = helpers.CloseAndClearSelection},
    {mods = "ALT", key = "i", action = act.CopyMode("CycleMatchType")},
    {mods = "ALT", key = "w", action = act.CopyMode("CycleMatchType")},
    {mods = "SHIFT", key = "Enter", action = act.CopyMode("PriorMatch")},
    {mods = "NONE", key = "Enter", action = act.CopyMode("NextMatch")},
    {mods = "NONE", key = "UpArrow", action = act.CopyMode("PriorMatch")},
    {mods = "NONE", key = "DownArrow", action = act.CopyMode("NextMatch")},
    -- Perform copy (and then close)
    {
        mods = "CTRL",
        key = "c",
        action = act.Multiple({
            act.CopyTo("Clipboard"),
            act.ClearSelection,
            act.CopyMode("Close"),
        }),
    },
}
config.key_tables.pane = {
    {mods = "NONE", key = "Escape", action = act.PopKeyTable},
    {mods = "ALT", key = "RightArrow", action = act.SplitHorizontal({})},
    {mods = "ALT", key = "DownArrow", action = act.SplitVertical({})},
    {mods = "NONE", key = "s", action = act.PaneSelect},
    {mods = "NONE", key = "f", action = act.TogglePaneZoomState},
    {mods = "NONE", key = "r", action = act.RotatePanes("Clockwise")},
    {mods = "NONE", key = "x", action = act.CloseCurrentTab({confirm = false})},
}

--------------------------------------------------------------------------------
-- Mouse bindings
--------------------------------------------------------------------------------
config.mouse_bindings = {
  {
    mods = "NONE",
    event = { Down = { streak = 3, button = "Left" } },
    action = wezterm.action.SelectTextAtMouseCursor "Line",
  },
  {
    mods = "CTRL",
    event = { Down = { streak = 1, button = "Left" } },
    action = wezterm.action.SelectTextAtMouseCursor "SemanticZone",
  },
}

--------------------------------------------------------------------------------
-- OS-specific settings
--------------------------------------------------------------------------------
config.default_domain = 'WSL:Ubuntu'

--------------------------------------------------------------------------------
-- Fiddle
--------------------------------------------------------------------------------

-- Export the configuration
return config
