Keybind Format Reference
========================

WezTerm (`~/dotfiles/home/wezterm/wezterm.lua`)
-----------------------------------------------

Docs: https://wezfurlong.org/wezterm/config/keys.html\
Key names reference:
https://wezfurlong.org/wezterm/config/keys.html#available-key-assignments

Entries in `config.keys = { ... }` as Lua tables:

```lua
{ mods = "CTRL", key = "c", action = act.CopyTo("Clipboard") },
{ mods = "CTRL|ALT", key = "r", action = act.ReloadConfiguration },
{ mods = "ALT", key = "k", action = act.QuickSelect },
{ mods = "SHIFT", key = "Enter", action = act.SendString("\x1b[13;2u") },
```

- **`mods`**: `"CTRL"`, `"ALT"`, `"SHIFT"`, `"SUPER"` — combine with `|` e.g.
  `"CTRL|SHIFT"`
- **`key`**: single char `"c"`, or named: `"Enter"`, `"UpArrow"`, `"PageUp"`,
  `"Home"`, `"End"`, `"-"`, `"="`, `"0"`, ``"`"``, `"\\"`, `"F1"`–`"F12"`
- **`action`**: `act.CopyTo("Clipboard")`, `act.PasteFrom("Clipboard")`,
  `act.ToggleFullScreen`, `act.QuickSelect`, `act.SendString("...")`,
  `act.ActivateCommandPalette`,
  `wezterm.action_callback(function(window, pane) ... end)`

`config.disable_default_key_bindings = true` is set, so only explicitly listed
bindings are active.

Zellij (`~/dotfiles/home/.config/zellij/config.kdl`)
----------------------------------------------------

Docs: https://zellij.dev/documentation/keybindings\
Actions reference: https://zellij.dev/documentation/keybindings-possible-actions

KDL `bind` declarations inside mode blocks:

```kdl
keybinds clear-defaults=true {
    shared_except "locked" {
        bind "Alt t" { NewTab; }
        bind "Alt left" { GoToPreviousTab; }
        bind "Ctrl Alt f" { SwitchToMode "entersearch"; SearchInput 0; }
        bind "Shift Enter" { Write 27 91 49 51 59 50 117; }
    }
    pane {
        bind "f" { ToggleFocusFullscreen; SwitchToMode "normal"; }
    }
}
```

- **Modifier syntax**: `"Ctrl"`, `"Alt"`, `"Shift"` as prefix with space
  separator: `"Ctrl Alt f"`, `"Alt up"`
- **Key names**: `"a"`–`"z"`, `"0"`–`"9"`, `"up"`, `"down"`, `"left"`,
  `"right"`, `"Enter"`, `"PageUp"`, `"PageDown"`, `"F2"`, ``"`"``, etc.
- **Mode blocks**: `shared_except "locked"` (most bindings go here), `pane`,
  `entersearch`, `resize`, `renamepane`, `renametab`
- **Pass-through raw bytes**: `Write 27 91 49 51 59 50 117;` (decimal escape
  sequence)

`clear-defaults=true` is set, so only explicitly listed bindings are active.

VS Code (`~/dotfiles/home/.config/Code/User/keybindings.json`)
--------------------------------------------------------------

Docs: https://code.visualstudio.com/docs/getstarted/keybindings\
When-clause contexts:
https://code.visualstudio.com/docs/getstarted/keybindings#_when-clause-contexts\
Default keybindings:
https://code.visualstudio.com/docs/getstarted/keybindings#_default-keybindings

JSON array of objects:

```json
[
    {
        "key": "alt+c c",
        "command": "git.commitAll",
        "when": "!inDebugMode && !terminalFocus"
    },
    { "key": "ctrl+shift+f", "command": "editor.action.formatDocument" },
    { "key": "ctrl+f4", "command": "-workbench.action.closeActiveEditor" }
]
```

- **`key`**: `"ctrl+c"`, `"alt+shift+f"`, chords space-separated: `"alt+c c"`
  (Alt+C, then C)
- **Modifiers**: `ctrl+`, `alt+`, `shift+` (lowercase, `+` separator)
- **`command`**: VS Code command ID
- **`when`**: optional context expression
- **Prefix `-`** on command: removes a default binding

Fish (`~/dotfiles/home/.config/fish/config.fish`)
-------------------------------------------------

Docs: https://fishshell.com/docs/current/cmds/bind.html\
Readline functions: https://fishshell.com/docs/current/interactive.html#readline

`bind` calls in config.fish:

```fish
bind ctrl-h backward-kill-word
bind alt-backspace backward-kill-line
bind up up-or-search
bind ctrl-delete kill-word
```

- **Modifier syntax**: `ctrl-`, `alt-` (lowercase, `-` separator, no space):
  `ctrl-h`, `alt-left`
- **Key names**: single chars, or `up`, `down`, `left`, `right`, `backspace`,
  `delete`, `enter`, `f1`–`f12`
- **Actions**: fish readline function names (`backward-kill-word`,
  `forward-word`, `up-or-search`, `beginning-of-line`, etc.) or custom fish
  function names

Fresh (`~/dotfiles/home/.config/fresh/config.json`)
---------------------------------------------------

Docs: https://fresh-editor.com/docs/keybindings (fetch if you need to look up
action names or context values)

JSON array in the `"keybinds"` field:

```json
{
    "keybinds": [
        { "key": "w", "modifiers": ["ctrl"], "action": "expand_selection" },
        {
            "key": "a",
            "modifiers": ["alt"],
            "action": "quick_open",
            "when": "global"
        },
        {
            "key": "PageDown",
            "modifiers": ["shift", "alt"],
            "action": "add_cursor_next_match"
        },
        {
            "key": "F7",
            "modifiers": ["alt", "shift"],
            "action": "noop",
            "when": "normal"
        }
    ]
}
```

- **`key`**: single char `"w"`, or named: `"Enter"`, `"PageUp"`, `"PageDown"`,
  `"Left"`, `"Right"`, `"F1"`–`"F12"`
- **`modifiers`**: array of strings: `["ctrl"]`, `["alt"]`, `["shift"]`, or
  combinations: `["ctrl", "alt"]`
- **`action`**: Fresh action name string
- **`when`**: optional context — `"global"`, `"normal"`, etc.

Claude Code (`~/dotfiles/home/.claude/keybindings.json`)
--------------------------------------------------------

Docs: https://docs.anthropic.com/en/docs/claude-code/keyboard-shortcuts

JSON array of context groups:

```json
[{
    "context": "Global",
    "bindings": {
        "alt+q": "app:quit",
        "ctrl+shift+b": "app:toggleBrief",
        "shift+enter": "chat:newline",
        "meta+p": null
    }
}]
```

- **Key syntax**: `"alt+q"`, `"ctrl+shift+b"`, `"shift+enter"` — modifiers
  `alt+`, `ctrl+`, `shift+`, `meta+` (lowercase, `+` separator)
- **Key names**: single chars, or `Enter`, `PageDown`, `Up`, `Left`, `Right`,
  `Backspace`, `Delete`
- **Value**: action string (e.g. `"app:quit"`, `"chat:newline"`) or `null` to
  disable
- **`context`**: `"Global"` for most bindings; other contexts exist for specific
  Claude Code modes
