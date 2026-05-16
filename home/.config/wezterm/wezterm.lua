local wezterm = require("wezterm")
local mux = wezterm.mux
local act = wezterm.action

local bayov_color_scheme = require("bayov_color_scheme")
local copy_mode = require("copy_mode")
local helpers = require("helpers")
require("search_mode")
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

-- Cursor
config.default_cursor_style = "BlinkingBlock"
config.cursor_blink_rate = 500
-- Misc
config.max_fps = 60
config.animation_fps = 60
-- Drives `update-status` cadence; lowered from 1000ms so copy/search-mode
-- background tints react quickly when entering/leaving the modes.
config.status_update_interval = 50
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
            -- Skip if already fullscreen (e.g. reattaching to a persisted
            -- mux session); otherwise the toggle would un-fullscreen it.
            if not gw:get_dimensions().is_full_screen then
                gw:toggle_fullscreen()
            end
        end
    end
end)

-- TODO: Better tabline
local tabline = require("tabline")
tabline.plugin.apply_to_config(config)
config.use_fancy_tab_bar = false
config.tab_bar_at_bottom = true
config.window_padding = {
    left = 0,
    right = 0,
    top = "0.5cell",
    bottom = 0,
}

--------------------------------------------------------------------------------
-- Keyboard bindings
--------------------------------------------------------------------------------
config.disable_default_key_bindings = true
config.keys = {
    -- Search / Copy mode. Pass Ctrl+F through to vim/fresh which bind it
    -- themselves (page-down / next-record); otherwise enter search mode.
    helpers.KeyUnlessApp({
        mods = "CTRL",
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
    }, { "fresh" }),
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
    helpers.KeyUnlessTui({ mods = "ALT", key = "[", action = act.ScrollToPrompt(-1) }),
    helpers.KeyUnlessTui({ mods = "ALT", key = "]", action = act.ScrollToPrompt(1) }),
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
    { mods = "ALT", key = "1", action = act.MoveTabRelative(-1) },
    { mods = "ALT", key = "3", action = act.MoveTabRelative(1) },
    { mods = "ALT|SHIFT", key = "x", action = act.CloseCurrentTab({ confirm = false }) },
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
    { mods = "ALT|SHIFT", key = "UpArrow", action = act.ActivatePaneDirection("Up") },
    { mods = "ALT|SHIFT", key = "DownArrow", action = act.ActivatePaneDirection("Down") },
    { mods = "ALT|SHIFT", key = "LeftArrow", action = act.ActivatePaneDirection("Left") },
    { mods = "ALT|SHIFT", key = "RightArrow", action = act.ActivatePaneDirection("Right") },
    { mods = "ALT|SHIFT", key = "8", action = act.SplitPane({ direction = "Up" }) },
    { mods = "ALT|SHIFT", key = "2", action = act.SplitPane({ direction = "Down" }) },
    { mods = "ALT|SHIFT", key = "4", action = act.SplitPane({ direction = "Left" }) },
    { mods = "ALT|SHIFT", key = "6", action = act.SplitPane({ direction = "Right" }) },
    { mods = "CTRL|ALT", key = "8", action = act.AdjustPaneSize({ "Up", 2 }) },
    { mods = "CTRL|ALT", key = "2", action = act.AdjustPaneSize({ "Down", 2 }) },
    { mods = "CTRL|ALT", key = "4", action = act.AdjustPaneSize({ "Left", 4 }) },
    { mods = "CTRL|ALT", key = "6", action = act.AdjustPaneSize({ "Right", 4 }) },
    { mods = "ALT", key = "9", action = act.RotatePanes("CounterClockwise") },
    { mods = "ALT", key = "7", action = act.RotatePanes("Clockwise") },
    { mods = "ALT", key = "*", action = act.PaneSelect({ mode = "SwapWithActive" }) },
    { mods = "ALT", key = "5", action = act.TogglePaneZoomState },
    -- TODO: Support re-opening closed pane (with grace period of 30 seconds).
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
    { mods = "NONE", key = "Escape", action = copy_mode.ClearSelectionOrCloseCopyMode },
    { mods = "ALT", key = "c", action = copy_mode.CloseAndClearSelection },
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
    -- Movement
    { mods = "NONE", key = "UpArrow",    action = copy_mode.Move("Up") },
    { mods = "NONE", key = "DownArrow",  action = copy_mode.Move("Down") },
    { mods = "NONE", key = "LeftArrow",  action = copy_mode.Move("Left") },
    { mods = "NONE", key = "RightArrow", action = copy_mode.Move("Right") },
    { mods = "CTRL", key = "LeftArrow",  action = copy_mode.Move("BackwardWord") },
    { mods = "CTRL", key = "RightArrow", action = copy_mode.Move("ForwardWord") },
    { mods = "NONE", key = "Home",       action = copy_mode.Move("ToStartOfLineContent") },
    { mods = "NONE", key = "End",        action = copy_mode.Move("ToEndOfLineContent") },
    { mods = "CTRL", key = "Home",       action = copy_mode.Move("ToScrollbackTop") },
    { mods = "CTRL", key = "End",        action = copy_mode.Move("ToScrollbackBottom") },
    { mods = "NONE", key = "PageUp",   action = copy_mode.Move({ MoveByPage = -0.5 }) },
    { mods = "NONE", key = "PageDown", action = copy_mode.Move({ MoveByPage = 0.5 }) },
    { mods = "CTRL", key = "[",        action = copy_mode.Move("BackwardSemanticZone") },
    { mods = "CTRL", key = "]",        action = copy_mode.Move("ForwardSemanticZone") },
    -- Extend selection (non-vim)
    { mods = "SHIFT", key = "LeftArrow",  action = copy_mode.ExtendSelection("Left") },
    { mods = "SHIFT", key = "RightArrow", action = copy_mode.ExtendSelection("Right") },
    { mods = "SHIFT", key = "UpArrow",    action = copy_mode.ExtendSelection("Up") },
    { mods = "SHIFT", key = "DownArrow",  action = copy_mode.ExtendSelection("Down") },
    { mods = "CTRL|SHIFT", key = "LeftArrow",  action = copy_mode.ExtendSelection("BackwardWord") },
    { mods = "CTRL|SHIFT", key = "RightArrow", action = copy_mode.ExtendSelection("ForwardWord") },
    { mods = "SHIFT", key = "Home",       action = copy_mode.ExtendSelection("ToStartOfLineContent") },
    { mods = "SHIFT", key = "End",        action = copy_mode.ExtendSelection("ToEndOfLineContent") },
    { mods = "CTRL|SHIFT", key = "Home",       action = copy_mode.ExtendSelection("ToScrollbackTop") },
    { mods = "CTRL|SHIFT", key = "End",        action = copy_mode.ExtendSelection("ToScrollbackBottom") },
    { mods = "SHIFT", key = "PageUp",   action = copy_mode.ExtendSelection({ MoveByPage = -0.5 }) },
    { mods = "SHIFT", key = "PageDown", action = copy_mode.ExtendSelection({ MoveByPage = 0.5 }) },
    { mods = "CTRL|SHIFT", key = "{",   action = copy_mode.ExtendSelection("BackwardSemanticZone") },
    { mods = "CTRL|SHIFT", key = "}",   action = copy_mode.ExtendSelection("ForwardSemanticZone") },
    -- Quick selection
    { mods = "CTRL", key = "l", action = act.CopyMode({ SetSelectionMode = "Line" }) },
    { mods = "CTRL", key = "s", action = act.CopyMode({ SetSelectionMode = "SemanticZone" }) },
    { mods = "CTRL", key = "w", action = act.CopyMode({ SetSelectionMode = "Word" }) },
    { mods = "CTRL", key = "b", action = act.CopyMode({ SetSelectionMode = "Block" }) },
}
-- Search mode reuses the copy mode bindings and only overrides/adds the
-- match-navigation keys. Escape, Ctrl+c, y, movement, etc. are inherited.
config.key_tables.search_mode = helpers.ExtendKeyTable(config.key_tables.copy_mode, {
    { mods = "ALT", key = "i", action = act.CopyMode("CycleMatchType") },
    { mods = "ALT", key = "u", action = act.CopyMode("CycleMatchType") },
    { mods = "ALT", key = "y", action = act.CopyMode("CycleMatchType") },
    { mods = "SHIFT", key = "Enter", action = act.CopyMode("PriorMatch") },
    { mods = "NONE", key = "Enter", action = act.CopyMode("NextMatch") },
})

--------------------------------------------------------------------------------
-- Mouse bindings
--------------------------------------------------------------------------------
config.mouse_bindings = {
    { mods = "NONE", event = { Down = { streak = 3, button = "Left" } }, action = act.SelectTextAtMouseCursor("Line") },
    { mods = "CTRL", event = { Down = { streak = 1, button = "Left" } }, action = act.SelectTextAtMouseCursor("SemanticZone") },
    -- Disable copy on selection
    { mods = "NONE", event = { Up = { streak = 1, button = "Left" } }, action = act.Nop },
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
-- The GUI connects to a `wezterm-mux-server` over a unix socket. The mux
-- server outlives the GUI, so closing/reopening WezTerm reattaches to the
-- same tabs/panes. Skipped on Windows, where the default domain is WSL
-- (see OS-specific settings below).
if not wezterm.target_triple:find("windows") then
    config.unix_domains = {
        { name = "unix" },
    }
    config.default_gui_startup_args = { "connect", "unix" }
end

--------------------------------------------------------------------------------
-- OS-specific settings
--------------------------------------------------------------------------------

if wezterm.target_triple:find("windows") then
    config.default_domain = "WSL:Ubuntu"
end

-- Export the configuration
return config
