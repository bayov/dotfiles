local wezterm = require("wezterm")
local act = wezterm.action

-- This will hold the module we export.
local module = {}

-- Copies selected text in the active pane to clipboard, or sends Ctrl+C
-- (SIGINT) to the terminal if no text is selected.
module.CopyOrInterrupt = wezterm.action_callback(function(window, pane)
    selection_text = window:get_selection_text_for_pane(pane)
    is_selection_active = string.len(selection_text) ~= 0
    if is_selection_active then
        window:perform_action(act.CopyTo("Clipboard"), pane)
        window:perform_action(act.ClearSelection, pane)
    else
        window:perform_action(act.SendKey({ mods = "CTRL", key = "c" }), pane)
    end
end)

-- Clears the current pane's viewport while preserving its content in
-- scrollback buffer. This is useful when you want to clear the screen
-- but keep the ability to scroll up to see previous output.
module.ClearPaneWhileKeepingScrollback = wezterm.action_callback(
    function(window, pane)
        -- avoid running in TUI programs
        if pane:is_alt_screen_active() then
            window:perform_action(
                act.SendKey({ mods = "ALT", key = "l" }),
                pane
            )
            return
        end

        -- scroll to bottom
        window:perform_action(act.ScrollToBottom, pane)

        -- get the current height of the viewport
        local height = pane:get_dimensions().viewport_rows

        -- inject a string of new lines equal to the viewport height
        -- local blank_viewport = string.rep("\r\n", height)
        -- pane:inject_output(blank_viewport)

        -- send an escape sequence to clear the viewport (CTRL-L)
        pane:send_text("\x0c")
    end
)

-- Registers an `update-status` handler that tints the window background to
-- `bg_color` and dims inactive panes while the given `key_table_name` is the
-- active key table. Only mutates overrides on transitions to avoid spamming
-- reloads on every status tick.
module.TintWhileKeyTableActive = function(key_table_name, bg_color)
    wezterm.on("update-status", function(window, _pane)
        local active = window:active_key_table() == key_table_name
        local overrides = window:get_config_overrides() or {}
        local currently_tinted = overrides.colors
            and overrides.colors.background == bg_color
        if active and not currently_tinted then
            overrides.colors = { background = bg_color }
            overrides.inactive_pane_hsb = { saturation = 0.7, brightness = 0.5 }
            window:set_config_overrides(overrides)
        elseif not active and currently_tinted then
            overrides.colors = nil
            overrides.inactive_pane_hsb = nil
            window:set_config_overrides(overrides)
        end
    end)
end

-- Returns a new key table built from `base` with `extras` appended.
-- Bindings in `extras` override bindings in `base` that share the same
-- mods+key combination.
module.ExtendKeyTable = function(base, extras)
    local function id(binding)
        return (binding.mods or "NONE") .. ":" .. binding.key
    end
    local overridden = {}
    for _, e in ipairs(extras) do
        overridden[id(e)] = true
    end
    local result = {}
    for _, e in ipairs(base) do
        if not overridden[id(e)] then
            table.insert(result, e)
        end
    end
    for _, e in ipairs(extras) do
        table.insert(result, e)
    end
    return result
end

-- Wraps a WezTerm `key_bind` (provided as `{mods, key, action}`) and returns
-- a new keybind where `action` is performed only when the foreground process
-- in the current pane is not in `apps` (list of basenames, e.g. {"vim",
-- "fresh"}). When the foreground process matches, sends the key as is so the
-- app receives it.
module.KeyUnlessApp = function(key_bind, apps)
    local mods = key_bind.mods
    local key = key_bind.key
    local action = key_bind.action
    local app_set = {}
    for _, name in ipairs(apps) do
        app_set[name] = true
    end

    return {
        mods = mods,
        key = key,
        action = wezterm.action_callback(function(window, pane)
            local proc = pane:get_foreground_process_name() or ""
            local basename = proc:match("([^/\\]+)$") or proc
            if app_set[basename] then
                window:perform_action(
                    act.SendKey({ key = key, mods = mods }),
                    pane
                )
            else
                window:perform_action(action, pane)
            end
        end),
    }
end

-- Wraps a WezTerm `key_bind` (provided as `{mods, key, action}`) and returns
-- a new keybind where `action` is performed only when the current pane is not
-- a TUI. When the pane is a TUI, sends the key as is instead.
module.KeyUnlessTui = function(key_bind)
    local mods = key_bind.mods
    local key = key_bind.key
    local action = key_bind.action

    return {
        mods = mods,
        key = key,
        action = wezterm.action_callback(function(window, pane)
            if not pane:is_alt_screen_active() then
                window:perform_action(action, pane)
            else
                window:perform_action(
                    act.SendKey({ key = key, mods = mods }),
                    pane
                )
            end
        end),
    }
end

module.RenameTab = act.PromptInputLine({
    description = "Rename tab: ",
    action = wezterm.action_callback(function(window, pane, line)
        if line then
            window:active_tab():set_title(line)
        end
    end),
})

-- Export the module
return module
