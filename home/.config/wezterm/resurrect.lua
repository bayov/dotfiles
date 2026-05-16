local wezterm = require("wezterm")

local module = {}

module.plugin =
    wezterm.plugin.require("https://github.com/MLFlexer/resurrect.wezterm")

-- Cap pane scrollback saved per state to keep files small and saves fast.
module.plugin.state_manager.set_max_nlines(5000)

-- Save workspace state to disk every 10 minutes. Tag every save with the
-- workspace name in `current_state` so the next mux startup knows what to
-- restore — save_state alone doesn't update that pointer.
module.plugin.state_manager.periodic_save({ interval_seconds = 600 })
wezterm.on("resurrect.state_manager.periodic_save.finished", function()
    local name = wezterm.mux.get_active_workspace() or "default"
    module.plugin.state_manager.write_current_state(name, "workspace")
end)

-- Auto-restore on first GUI attach after a fresh mux-server start.
--
-- mux-startup runs in the mux-server process; gui-attached runs in the GUI
-- process. They don't share Lua state, so we hand off via a flag file:
--   - mux-startup writes the flag (mux just (re)started → restore is wanted)
--   - first gui-attached deletes the flag and performs restore_window into
--     the attached window with close_open_tabs=true, replacing the default
--     starter tab. Subsequent attaches see no flag and do nothing.
--
-- Set WEZTERM_NO_RESURRECT=1 in the environment to skip auto-restore.
local flag_path = module.plugin.state_manager.save_state_dir .. "auto-restore-pending"

wezterm.on("mux-startup", function()
    if os.getenv("WEZTERM_NO_RESURRECT") == "1" then return end
    local f = io.open(flag_path, "w")
    if f then f:write("1") f:close() end
end)

wezterm.on("gui-attached", function()
    local f = io.open(flag_path, "r")
    if not f then return end
    f:close()
    os.remove(flag_path) -- one-shot

    local sm = module.plugin.state_manager
    local pf = io.open(sm.save_state_dir .. "current_state", "r")
    if not pf then return end
    local name = pf:read("*line")
    local kind = pf:read("*line")
    pf:close()
    if kind ~= "workspace" or not name or name == "" then return end

    local state = sm.load_state(name, "workspace")
    if not state.window_states or #state.window_states == 0 then return end

    -- Defer the actual restore so any later-registered gui-attached listeners
    -- (e.g. the fullscreen toggle in wezterm.lua) run first. Splitting panes
    -- against the post-fullscreen window size keeps proportional layouts
    -- correct; otherwise panes get baked into the 120x28 starter dimensions.
    wezterm.time.call_after(0.3, function()
        local gw = wezterm.gui.gui_windows()[1]
        if not gw then return end
        local mux_win = gw:mux_window()

        local restore_opts = {
            relative = true,
            restore_text = true,
            close_open_tabs = true,
            on_pane_restore = module.plugin.tab_state.default_on_pane_restore,
        }
        module.plugin.window_state.restore_window(
            mux_win, state.window_states[1], restore_opts
        )

        for i = 2, #state.window_states do
            local ws = state.window_states[i]
            local _, _, extra_win = wezterm.mux.spawn_window({
                workspace = state.workspace,
                cwd = ws.tabs[1].pane_tree.cwd,
                width = ws.size.cols,
                height = ws.size.rows,
            })
            module.plugin.window_state.restore_window(extra_win, ws, {
                relative = true,
                restore_text = true,
                close_open_tabs = true,
                on_pane_restore = module.plugin.tab_state.default_on_pane_restore,
            })
        end

        wezterm.mux.set_active_workspace(state.workspace)
        wezterm.log_info("resurrect: auto-restored workspace " .. state.workspace)
    end)
end)

-- Save the active workspace.
module.save_workspace = wezterm.action_callback(function(window, pane)
    local state = module.plugin.workspace_state.get_workspace_state()
    module.plugin.state_manager.save_state(state)
    module.plugin.state_manager.write_current_state(state.workspace, "workspace")
    wezterm.log_info("resurrect: workspace saved (" .. state.workspace .. ")")
    window:toast_notification("resurrect", "workspace saved", nil, 2000)
end)

-- Fuzzy-pick a saved workspace/window/tab and restore it.
module.restore_state = wezterm.action_callback(function(window, pane)
    module.plugin.fuzzy_loader.fuzzy_load(window, pane, function(id, label)
        local kind = string.match(id, "^([^/]+)")
        id = string.match(id, "([^/]+)$")
        id = string.match(id, "(.+)%..+$")
        local opts = {
            relative = true,
            restore_text = true,
            on_pane_restore =
                module.plugin.tab_state.default_on_pane_restore,
        }
        if kind == "workspace" then
            local state = module.plugin.state_manager.load_state(id, "workspace")
            module.plugin.workspace_state.restore_workspace(state, opts)
        elseif kind == "window" then
            local state = module.plugin.state_manager.load_state(id, "window")
            module.plugin.window_state.restore_window(window:mux_window(), state, opts)
        elseif kind == "tab" then
            local state = module.plugin.state_manager.load_state(id, "tab")
            module.plugin.tab_state.restore_tab(pane:tab(), state, opts)
        end
    end)
end)

return module
