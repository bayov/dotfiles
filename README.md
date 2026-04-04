Personal dotfiles managed with a custom symlinker and shell install scripts.

Install
-------

```sh
# Install all packages and tools
./dev/install.sh

# Symlink home/ files to their destinations in $HOME
./dev/link.py
```

**Manually install on Windows**

- AutoHotKey 2.0 — https://www.autohotkey.com/
- WezTerm — https://wezterm.org/install/windows.html
- JetBrains Mono (font) — https://www.jetbrains.com/lp/mono/

Structure
---------

| Directory  | Contents                                                               |
| ---------- | ---------------------------------------------------------------------- |
| `home/`    | Config files — symlinked into `$HOME` by `dev/link.py`                 |
| `dev/`     | Install scripts and the symlinker (`link.py`)                          |
| `pybayov/` | Python utility package used by shell aliases and scripts               |
| `ahk/`     | AutoHotkey 2.0 scripts for Windows automation                          |
| `config/`  | Misc config files (commit template, AI model list, sandbox Dockerfile) |
| `notes/`   | Personal notes                                                         |

Development
-----------

Run `bayov-format` before committing and `bayov-test` before pushing:

```sh
bayov-format   # alias: precommit
bayov-test     # alias: prepush --quiet --show-failures
```

What gets configured
--------------------

- **Shell**: Fish 4+ with fzf integration, custom functions, and Starship prompt
- **Terminal**: WezTerm (custom Lua config, color scheme, tab bar)
- **Multiplexer**: Zellij
- **VCS**: Jujutsu (`jj`) as primary, Git with delta pager
- **Editor**: Fresh (`$EDITOR`), VS Code
- **AI tools**: Claude Code, Gemini CLI, OpenCode
- **Language toolchains**: Rust (nightly), Node.js (via fnm), Python (via uv)
