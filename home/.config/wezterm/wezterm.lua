local wezterm = require("wezterm")
local mux = wezterm.mux
local act = wezterm.action

local bayov_color_scheme = require("bayov_color_scheme")
local helpers = require("helpers")
local zoom = require("zoom")

-- This will hold the configuration we export.
local config = wezterm.config_builder()

--------------------------------------------------------------------------------
-- Scrollback
--------------------------------------------------------------------------------
config.scrollback_lines = 100000
config.enable_scroll_bar = false

--------------------------------------------------------------------------------
-- Theme & Appearance
--------------------------------------------------------------------------------
-- Font
config.font = wezterm.font("JetBrains Mono NL")
config.line_height = 1.0
if wezterm.target_triple:find("darwin") then
    config.font_size = 14
else
    config.font_size = 10
end
wezterm.GLOBAL.base_font_size = config.font_size
config.harfbuzz_features = { "calt=0", "clig=0", "liga=0" } -- disable ligatures
-- Colors
bayov_color_scheme.apply_to_config(config)
config.inactive_pane_hsb = {
    saturation = 0.9,
    brightness = 0.8,
}
-- Window
config.window_decorations = "TITLE|RESIZE"
config.initial_cols = 120
config.initial_rows = 28
config.window_padding = {
    left = "1cell",
    right = "1cell",
    top = "0.5cell",
    bottom = "0",
}

-- Cursor
config.default_cursor_style = "BlinkingBlock"
config.cursor_blink_rate = 500
-- Misc
config.max_fps = 120
config.animation_fps = 120
config.window_close_confirmation = "NeverPrompt"
config.disable_default_quick_select_patterns = true
local url_regex =
    "(https?:\\/\\/(?:www\\.)?[-a-zA-Z0-9@:%._\\+~#=]{1,256}\\.[a-zA-Z0-9()]{1,6}\b(?:[-a-zA-Z0-9()@:%_\\+.~#?&\\/=]*))"
config.quick_select_patterns = {
    url_regex,
    -- Base16 IDs
    "[0-9a-f]{8,40}",
    -- jj change IDs
    --
    -- Note:
    -- Technically, WezTerm docs say that capture groups should not be used as
    -- the patterns above are compiled into a single regex and these groups
    -- might interfere. However, I found that the above just works for now.
    "[@◆○×│] {2,}([a-z]{8})",
}
config.quick_select_alphabet = "qwerasdzxcv"

-- Start in full-screen on the primary display (the one with the menu bar).
-- Without set_position, macOS spawns the window on whichever display had
-- focus / the cursor — toggling fullscreen there pins it to the wrong space.
wezterm.on("gui-attached", function(window, pane)
    local main_screen = wezterm.gui.screens().main
    local workspace = mux.get_active_workspace()
    for _, w in ipairs(mux.all_windows()) do
        if w:get_workspace() == workspace then
            local gw = w:gui_window()
            gw:set_position(main_screen.x, main_screen.y)
            gw:toggle_fullscreen()
        end
    end
end)

local tabline = require("tabline")
tabline.plugin.apply_to_config(config)
config.use_fancy_tab_bar = false
config.tab_bar_at_bottom = true

--------------------------------------------------------------------------------
-- Keyboard bindings
--------------------------------------------------------------------------------
config.disable_default_key_bindings = true
config.keys = {
    -- Search / Copy mode
    {
        mods = "CTRL|ALT",
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
        end),
    },
    { mods = "ALT", key = "c", action = act.ActivateCopyMode },

    -- Copy & Paste
    { mods = "CTRL", key = "c", action = helpers.CopyOrInterrupt },
    { mods = "CTRL", key = "v", action = act.PasteFrom("Clipboard") },

    -- Scrolling & Scrollback
    helpers.KeyUnlessTui({ mods = "ALT", key = "UpArrow", action = act.ScrollByLine(-2) }),
    helpers.KeyUnlessTui({ mods = "ALT", key = "DownArrow", action = act.ScrollByLine(2) }),
    helpers.KeyUnlessTui({ mods = "ALT", key = "PageUp", action = act.ScrollByPage(-0.5) }),
    helpers.KeyUnlessTui({ mods = "ALT", key = "PageDown", action = act.ScrollByPage(0.5) }),
    helpers.KeyUnlessTui({ mods = "ALT", key = "Home", action = act.ScrollToTop }),
    helpers.KeyUnlessTui({ mods = "ALT", key = "End", action = act.ScrollToBottom }),
    helpers.KeyUnlessTui({ mods = "CTRL", key = "l", action = helpers.ClearPaneWhileKeepingScrollback }),
    helpers.KeyUnlessTui({
        mods = "CTRL|ALT",
        key = "l",
        action = act.Multiple({
            act.ClearScrollback("ScrollbackAndViewport"),
            act.SendKey({ mods = "CTRL", key = "L" }),
        }),
    }),

    -- Presentation
    { mods = "ALT", key = "\\", action = act.ToggleFullScreen },
    { mods = "CTRL", key = "-", action = zoom.ZoomOut },
    { mods = "CTRL", key = "=", action = zoom.ZoomIn },
    { mods = "CTRL", key = "0", action = zoom.ResetZoom },

    -- Tab-management
    { mods = "ALT", key = "t", action = act.SpawnTab("DefaultDomain") },
    { mods = "ALT", key = "LeftArrow", action = act.ActivateTabRelative(-1) },
    { mods = "ALT", key = "RightArrow", action = act.ActivateTabRelative(1) },
    { mods = "ALT", key = "[", action = act.MoveTabRelative(-1) },
    { mods = "ALT", key = "]", action = act.MoveTabRelative(1) },
    {
        mods = "NONE",
        key = "F2",
        action = act.PromptInputLine({
            description = "Rename tab: ",
            action = wezterm.action_callback(function(window, pane, line)
                if line then
                    window:active_tab():set_title(line)
                end
            end),
        }),
    },

    -- Pane-management
    { mods = "ALT", key = "s", action = act.ActivateKeyTable({ name = "pane" }) },
    { mods = "ALT|SHIFT", key = "UpArrow", action = act.ActivatePaneDirection("Up") },
    { mods = "ALT|SHIFT", key = "DownArrow", action = act.ActivatePaneDirection("Down") },
    { mods = "ALT|SHIFT", key = "LeftArrow", action = act.ActivatePaneDirection("Left") },
    { mods = "ALT|SHIFT", key = "RightArrow", action = act.ActivatePaneDirection("Right") },
    { mods = "ALT|SHIFT", key = "8", action = act.SplitPane({ direction = "Up" }) },
    { mods = "ALT|SHIFT", key = "2", action = act.SplitPane({ direction = "Down" }) },
    { mods = "ALT|SHIFT", key = "4", action = act.SplitPane({ direction = "Left" }) },
    { mods = "ALT|SHIFT", key = "6", action = act.SplitPane({ direction = "Right" }) },
    { mods = "CTRL|ALT", key = "8", action = act.AdjustPaneSize({ "Up", 4 }) },
    { mods = "CTRL|ALT", key = "2", action = act.AdjustPaneSize({ "Down", 4 }) },
    { mods = "CTRL|ALT", key = "4", action = act.AdjustPaneSize({ "Left", 4 }) },
    { mods = "CTRL|ALT", key = "6", action = act.AdjustPaneSize({ "Right", 4 }) },
    { mods = "ALT|SHIFT", key = "{", action = act.RotatePanes("CounterClockwise") },
    { mods = "ALT|SHIFT", key = "}", action = act.RotatePanes("Clockwise") },
    { mods = "ALT", key = "x", action = act.CloseCurrentPane({ confirm = false }) },

    -- Misc
    { mods = "CTRL|ALT", key = "a", action = act.ActivateCommandPalette },
    { mods = "CTRL|ALT", key = "r", action = act.ReloadConfiguration },
    { mods = "CTRL", key = "`", action = act.ShowDebugOverlay },

    -- Support Shift+Enter for CLI tools
    { mods = "SHIFT", key = "Enter", action = act.SendString("\x1b[13;2u") },

    -- QuickSelect
    { mods = "ALT", key = "k", action = act.QuickSelect },
    {
        mods = "ALT",
        key = "o",
        action = act.QuickSelectArgs({
            label = "open url",
            patterns = { url_regex },
            action = wezterm.action_callback(function(window, pane)
                local url = window:get_selection_text_for_pane(pane)
                wezterm.log_info("opening: " .. url)
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
    { mods = "NONE", key = "Escape", action = helpers.CloseAndClearSelection },
    { mods = "ALT", key = "c", action = helpers.CloseAndClearSelection },
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
    {
        mods = "NONE",
        key = "v",
        action = act.CopyMode({ SetSelectionMode = "Cell" }),
    },
    {
        mods = "SHIFT",
        key = "v",
        action = act.CopyMode({ SetSelectionMode = "Line" }),
    },
    {
        mods = "CTRL",
        key = "v",
        action = act.CopyMode({ SetSelectionMode = "SemanticZone" }),
    },
    {
        mods = "CTRL",
        key = "w",
        action = act.CopyMode({ SetSelectionMode = "Word" }),
    },
    {
        mods = "SHIFT",
        key = "Insert",
        action = act.CopyMode({ SetSelectionMode = "Block" }),
    },
    -- Movement
    { mods = "NONE", key = "UpArrow", action = act.CopyMode("MoveUp") },
    { mods = "NONE", key = "DownArrow", action = act.CopyMode("MoveDown") },
    { mods = "NONE", key = "LeftArrow", action = act.CopyMode("MoveLeft") },
    { mods = "NONE", key = "RightArrow", action = act.CopyMode("MoveRight") },
    {
        mods = "CTRL",
        key = "LeftArrow",
        action = act.CopyMode("MoveBackwardWord"),
    },
    {
        mods = "CTRL",
        key = "RightArrow",
        action = act.CopyMode("MoveForwardWord"),
    },
    {
        mods = "NONE",
        key = "Home",
        action = act.CopyMode("MoveToStartOfLineContent"),
    },
    {
        mods = "NONE",
        key = "End",
        action = act.CopyMode("MoveToEndOfLineContent"),
    },
    {
        mods = "CTRL",
        key = "Home",
        action = act.CopyMode("MoveToScrollbackTop"),
    },
    {
        mods = "CTRL",
        key = "End",
        action = act.CopyMode("MoveToScrollbackBottom"),
    },
    {
        mods = "NONE",
        key = "PageUp",
        action = act.CopyMode({ MoveByPage = -0.5 }),
    },
    {
        mods = "NONE",
        key = "PageDown",
        action = act.CopyMode({ MoveByPage = 0.5 }),
    },
    {
        mods = "SHIFT",
        key = "UpArrow",
        action = act.CopyMode("MoveBackwardSemanticZone"),
    },
    {
        mods = "SHIFT",
        key = "DownArrow",
        action = act.CopyMode("MoveForwardSemanticZone"),
    },
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
    { mods = "NONE", key = "Escape", action = helpers.CloseAndClearSelection },
    { mods = "CTRL|ALT", key = "f", action = helpers.CloseAndClearSelection },
    { mods = "ALT", key = "i", action = act.CopyMode("CycleMatchType") },
    { mods = "ALT", key = "w", action = act.CopyMode("CycleMatchType") },
    { mods = "SHIFT", key = "Enter", action = act.CopyMode("PriorMatch") },
    { mods = "NONE", key = "Enter", action = act.CopyMode("NextMatch") },
    { mods = "NONE", key = "UpArrow", action = act.CopyMode("PriorMatch") },
    { mods = "NONE", key = "DownArrow", action = act.CopyMode("NextMatch") },
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
    { mods = "NONE", key = "Escape", action = act.PopKeyTable },
    { mods = "ALT", key = "s", action = act.PopKeyTable },
    -- Split in direction
    { mods = "ALT", key = "LeftArrow", action = act.SplitPane({ direction = "Left" }) },
    { mods = "ALT", key = "RightArrow", action = act.SplitPane({ direction = "Right" }) },
    { mods = "ALT", key = "UpArrow", action = act.SplitPane({ direction = "Up" }) },
    { mods = "ALT", key = "DownArrow", action = act.SplitPane({ direction = "Down" }) },
    -- Focus
    { mods = "NONE", key = "LeftArrow", action = act.ActivatePaneDirection("Left") },
    { mods = "NONE", key = "RightArrow", action = act.ActivatePaneDirection("Right") },
    { mods = "NONE", key = "UpArrow", action = act.ActivatePaneDirection("Up") },
    { mods = "NONE", key = "DownArrow", action = act.ActivatePaneDirection("Down") },
    -- Rotate (closest WezTerm has to Zellij's "move pane")
    { mods = "SHIFT", key = "LeftArrow", action = act.RotatePanes("CounterClockwise") },
    { mods = "SHIFT", key = "RightArrow", action = act.RotatePanes("Clockwise") },
    { mods = "SHIFT", key = "UpArrow", action = act.RotatePanes("CounterClockwise") },
    { mods = "SHIFT", key = "DownArrow", action = act.RotatePanes("Clockwise") },
    -- Resize
    { mods = "NONE", key = "8", action = act.AdjustPaneSize({ "Up", 4 }) },
    { mods = "NONE", key = "2", action = act.AdjustPaneSize({ "Down", 4 }) },
    { mods = "NONE", key = "4", action = act.AdjustPaneSize({ "Left", 4 }) },
    { mods = "NONE", key = "6", action = act.AdjustPaneSize({ "Right", 4 }) },
    -- Misc
    { mods = "NONE", key = "s", action = act.PaneSelect },
    { mods = "NONE", key = "f", action = act.TogglePaneZoomState },
    { mods = "NONE", key = "r", action = act.RotatePanes("Clockwise") },
    { mods = "NONE", key = "x", action = act.CloseCurrentPane({ confirm = false }) },
}

--------------------------------------------------------------------------------
-- Mouse bindings
--------------------------------------------------------------------------------
config.mouse_bindings = {
    {
        mods = "NONE",
        event = { Down = { streak = 3, button = "Left" } },
        action = wezterm.action.SelectTextAtMouseCursor("Line"),
    },
    {
        mods = "CTRL",
        event = { Down = { streak = 1, button = "Left" } },
        action = wezterm.action.SelectTextAtMouseCursor("SemanticZone"),
    },
    -- Disable copy on selection
    {
        mods = "NONE",
        event = { Up = { streak = 1, button = "Left" } },
        action = wezterm.action.Nop,
    },
    -- Paste with right-click (or copy if text is selected):
    {
        mods = "NONE",
        event = { Down = { streak = 1, button = "Right" } },
        action = wezterm.action_callback(function(window, pane)
            local has_selection = window:get_selection_text_for_pane(pane) ~= ""
            if has_selection then
                window:perform_action(
                    act.CopyTo("ClipboardAndPrimarySelection"),
                    pane
                )
                window:perform_action(act.ClearSelection, pane)
            else
                window:perform_action(act({ PasteFrom = "Clipboard" }), pane)
            end
        end),
    },
}

--------------------------------------------------------------------------------
-- Multiplexing
--------------------------------------------------------------------------------
-- Multiplexing disabled as I've moved to Zellij
-- config.unix_domains = {
--     { name = 'unix' },
-- }
-- config.default_prog = { 'wsl', '~' }
-- config.default_gui_startup_args = { 'connect', 'unix' }

--------------------------------------------------------------------------------
-- OS-specific settings
--------------------------------------------------------------------------------

if wezterm.target_triple:find("windows") then
    config.default_domain = "WSL:Ubuntu"
end

-- Export the configuration
return config
