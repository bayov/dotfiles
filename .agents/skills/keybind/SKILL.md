---
name: keybind
description: Manages keyboard shortcuts across all apps in this dotfiles setup.
disable-model-invocation: true
model: haiku
---

Keybind Manager
===============

This skill handles keybinding changes across all apps in this dotfiles setup.
The main challenge is the **terminal shadowing stack**: keys flow through
WezTerm → Zellij → running app (Fish/Fresh/Claude Code), and a binding at a
higher layer silently consumes the key before lower layers see it.

Config file locations
---------------------

| App             | Config file                                          | Context                                     |
| --------------- | ---------------------------------------------------- | ------------------------------------------- |
| **WezTerm**     | `~/dotfiles/home/wezterm/wezterm.lua`                | Terminal GUI (highest priority in terminal) |
| **Zellij**      | `~/dotfiles/home/.config/zellij/config.kdl`          | Multiplexer (second priority in terminal)   |
| **VS Code**     | `~/dotfiles/home/.config/Code/User/keybindings.json` | Standalone GUI app                          |
| **Fish**        | `~/dotfiles/home/.config/fish/config.fish`           | Shell (lowest priority in terminal)         |
| **Fresh**       | `~/dotfiles/home/.config/fresh/config.json`          | Editor (lowest priority in terminal)        |
| **Claude Code** | `~/dotfiles/home/.claude/keybindings.json`           | CLI tool (lowest priority in terminal)      |

Terminal shadowing
------------------

When using a terminal-based app (Fish, Fresh, Claude Code), keys pass through:

```
Keyboard → WezTerm → Zellij → Running app
```

If WezTerm binds `Alt+k`, that key never reaches Zellij or any running app. If
Zellij binds `Alt+k`, it never reaches Fish/Fresh/Claude Code. This is a
frequent source of silent bugs — the key appears to do nothing because it's
consumed upstream.

**When proposing changes:**

- Always check all three terminal layers, even if the user only asked about one
- Warn prominently if the requested key is already bound at a higher layer
- Sometimes the right fix is to remove/pass-through the higher-layer binding
  rather than adding a new one

Workflow
--------

1. Read the relevant config files to understand current bindings
2. Check for shadowing conflicts across the terminal stack
3. Propose the specific changes needed in each config, in the correct syntax
4. Flag any shadowing issues clearly
5. Apply changes after confirmation

Read `references/formats.md` for exact syntax and documentation links for each
app. **Fetch the relevant docs URL** when you need to look up valid action
names, key name constants, `when`-clause contexts, or any other details not
covered in the reference — do not guess.

Common patterns
---------------

**Adding a binding to a terminal app (Fish/Fresh/Claude Code):** First check if
the key is free in WezTerm and Zellij. If not, you'll need to either unbind it
upstream or pick a different key.

**Adding a Zellij binding:** Check if WezTerm already captures that key.
WezTerm's `config.keys` list is the authority — if the key isn't there and
`disable_default_key_bindings = true`, it passes through.

**Passing a key through Zellij to the running app:** In Zellij KDL, use `Write`
with the raw bytes of the key sequence, e.g.:

```kdl
bind "Shift Enter" { Write 27 91 49 51 59 50 117; }
```

**VS Code bindings** are completely independent of the terminal stack — they're
their own GUI and don't interact with WezTerm/Zellij.

**Chord bindings** (key sequences like `Alt+C then C`) are supported in VS Code
and WezTerm but not in Zellij or Fish.
