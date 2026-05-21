# Preferred Keymap

My preferred keyboard bindings to use in apps, where it makes sense and when
supported.

## Tab and pane management

> Note: Terminology in various apps: Pane == Split == Editor

Theme: Numpad is dedicated to spatial manipulation of tabs and panes. The
modifier picks the operation; the numpad key picks the direction (matching its
physical position on the keypad).

- `N2`, `N4`, `N6`, `N8` — direction: down, left, right, up (their physical
  position).
- `N5` — "center".
- `N1`, `N3` - secondary left and right
- `N-` — "remove"

**Important**: Avoid Shift modifier, as in Windows, a Shift+Numpad press is
converted to a Numpad press with NumLock toggled on/off.

| Key                    | Action                                     |
| ---------------------- | ------------------------------------------ |
| Alt+Arrows             | Move between tabs                          |
| Alt+Shift+Arrows       | Move between panes                         |
| Ctrl+Tab               | Tab switcher (menu)                        |
| Alt+{N2,N4,N6,N8}      | Move pane (split if direction empty)       |
| Ctrl+Alt+{N2,N4,N6,N8} | Stretch/resize pane                        |
| Ctrl+N5                | Toggle maximize pane                       |
| Alt+{N1,N3}            | Move tab left/right (within pane)          |
| Alt+{N7,N9}            | Rotate panes (clockwise/counter-clockwise) |
| Ctrl+Alt+N5            | Normalize/Equalize pane widths             |
| F2                     | Rename tab                                 |
| Alt+T                  | New tab                                    |
| Alt+P                  | Pin active tab                             |
| Alt+X                  | Close active tab                           |
| Alt+Shift+X            | Re-open last closed tab                    |
| Alt+N-                 | Close other tabs                           |
| Ctrl-Alt+N-            | Close all tabs in group                    |

## Multi-caret and Rectangular Selection

| Key                        | Action                                                                |
| -------------------------- | --------------------------------------------------------------------- |
| Shift+Insert               | Toggle column selection mode                                          |
| Alt+Click                  | Add Caret                                                             |
| Alt+Click                  | Add Rectangular Selection (mouse drag)                                |
| Alt+Ctrl+{Up,Down}         | Clone caret above/below                                               |
| Ctrl+Alt+PgDn              | Multi-select current selection / word under cursor                    |
| Ctrl+Alt+PgUp              | Undo last multi-select                                                |
| Ctrl+Alt+End               | Multi-select all matches                                              |
| Ctrl+Alt+Shift+{PgUp,PgDn} | Skip current selection (multi-select) / prev-next match (find/search) |

## Caret movement and selection

| Key                    | Action                                               |
| ---------------------- | ---------------------------------------------------- |
| Alt+{Up,Down}          | Scroll editor up/down 2 lines                        |
| Alt+{PgUp,PgDn}        | Scroll editor up/down 25 lines / half-page           |
| Alt+{Home,End}         | Scroll editor to top/bottom                          |
| Ctrl+{Left,Right}      | Move caret left/right one word                       |
| Ctrl+{Up,Down}         | Move caret up/down one structural element / "method" |
| {PgUp,PgDn}            | Move caret up/down half a page                       |
| Shift+{PgUp,PgDn}      | Move caret up/down half a page with selection        |
| Ctrl+{PgUp,PgDn}       | Move caret a Paragraph                               |
| Ctrl+Shift+{PgUp,PgDn} | Move caret a Paragraph with selection                |
| Ctrl+{[,]}             | Move caret a Block                                   |
| Ctrl+Shift+{[,]}       | Move caret a Block with selection                    |
| Ctrl+B                 | Move caret to matching brace                         |
| Ctrl+L                 | Select entire line                                   |
| Ctrl+W                 | Extend selection                                     |
| Ctrl+Shift+W           | Shrink selection                                     |
| Ctrl+G                 | Goto line                                            |
| Alt+{<,>}              | Previous/Next change in file                         |
| Ctrl+{<,>}             | Previous/Next problem in file                        |
| Ctrl+Shift+{<,>}       | Previous/Next problem in all files                   |
| Ctrl+Alt+Shift+<Arrow> | Scroll current window/pane/editor                    |

## Search and Replace

| Key          | Action                                                |
| ------------ | ----------------------------------------------------- |
| Shift+Shift  | Search everywhere                                     |
| Ctrl+Ctrl    | Search run configurations (run anything)              |
| Alt+A        | Search actions                                        |
| Alt+F        | Search files                                          |
| Alt+T        | Search types                                          |
| Ctrl+F       | Search editor (and toggle search in selection filter) |
| Ctrl+Shift+F | Find in all files                                     |
| Ctrl+Shift+R | Replace in all files                                  |
| Alt+I        | Toggle case-sensitivity                               |
| Alt+U        | Toggle whole-word search                              |
| Alt+Y        | Toggle regex-search                                   |

## Information / Quick navigation

| Key             | Action                                                          |
| --------------- | --------------------------------------------------------------- |
| Alt+Enter       | Show context actions (at caret)                                 |
| Ctrl+E          | Quick definition                                                |
| Ctrl+Shift+E    | Quick type definition                                           |
| Ctrl+Q          | Quick documentation                                             |
| Ctrl+P          | Parameter info                                                  |
| Ctrl+T          | Type info (might not be needed if "Quick documentation" is set) |
| Alt+Shift+Enter | Error info                                                      |
| Alt+G           | Go to declaration or usage                                      |
| Alt+Shift+G     | Go to implementations                                           |
| Alt+Ctrl+G      | Go to super method                                              |
| Alt+H           | Find usages (quick window)                                      |
| Alt+Shift+H     | Find usages (full window)                                       |
| Ctrl+Alt+H      | Highlight usages in file                                        |

## Navigation

| Key                   | Action                                         |
| --------------------- | ---------------------------------------------- |
| Alt+Home              | Focus breadcrumb                               |
| Ctrl+Alt+{Left,Right} | Back/Forward (last caret location)             |
| Ctrl+Shift+H          | Recent files (press again for edited only)     |
| Ctrl+Alt+Shift+H      | Recent locations (press again for edited only) |

## VCS

| Key            | Action                                                  |
| -------------- | ------------------------------------------------------- |
| Alt+W          | Switch workspace                                        |
| Alt+C, A       | All VCS Operations                                      |
| Alt+C, R       | Refresh (if GUI is out of sync due to external changes) |
| Alt+C, C       | Commit                                                  |
| Alt+C, P       | Update project (pull/sync)                              |
| Alt+C, D       | Compare current file with repo                          |
| Alt+C, Shift+D | Compare with revision....                               |
| Alt+C, B       | Blame / Annotate                                        |
| Alt+C, L       | Show VCS history                                        |
| Alt+C, Shift+L | Show VCS history for selection                          |
| Alt+C, Z       | Rollback / Revert / Reset                               |
| Alt+C, U       | Push / Update                                           |
| Alt+C, F       | Fetch                                                   |
| Alt+C, X       | Resolve conflicts                                       |
| Alt+C, S       | Shelve / Stash                                          |

## Misc

| Key         | Action                           |
| ----------- | -------------------------------- |
| Alt+L       | Show local history               |
| Alt+Shift+L | Show local history for selection |

## Presentation

| Key         | Action                  |
| ----------- | ----------------------- |
| Ctrl+{-,=}  | Zoom in and out         |
| Ctrl+0      | Reset zoom              |
| Alt+\       | Toggle full screen mode |
| Alt+Shift+P | Presentation mode       |
| Alt+Shift+Z | Zen mode                |

## Window management

| Key         | Action                                         |
| ----------- | ---------------------------------------------- |
| Alt+'       | Settings                                       |
| Alt+;       | Keymapping                                     |
| Alt+/       | Extensions                                     |
| Alt+E       | Focus editor                                   |
| Alt+Shift+E | Hide all tool windows                          |
| Alt+0       | Toggle toolbar icons (at the edges) visibility |
| Alt+-       | Toggle status bar (bottom) visibility          |

## Copy-Paste

| Key          | Action             |
| ------------ | ------------------ |
| Ctrl+Shift+X | Cut to line end    |
| Ctrl+Shift+V | Paste from history |

## Deletion

| Key             | Action                                    |
| --------------- | ----------------------------------------- |
| Ctrl+Delete     | Delete to word end                        |
| Ctrl+Backspace  | Delete to word start                      |
| Shift+Backspace | Hungry backspace (deletes all whitespace) |

## Modification

| Key                   | Action                      |
| --------------------- | --------------------------- |
| Ctrl+Alt+X            | Join lines                  |
| Ctrl+Alt+D            | Duplicate line or selection |
| Ctrl+Alt+C            | Toggle case                 |
| Ctrl+Alt+R            | Reverse lines               |
| Ctrl+Alt+T            | Sort lines                  |
| Ctrl+Shift+{Up,Down}  | Move line up/down           |
| Alt+Shift+{PgUp,PgDn} | Move statement up/down      |
| Ctrl+/                | Comment line / selection    |

## Refactor / Cleanup

| Key        | Action                                   |
| ---------- | ---------------------------------------- |
| Alt+D      | Show git panel / diffs in current commit |
| Ctrl+Alt+O | Optimize imports                         |
| `<none>`   | Rearrange code                           |
| F2         | Rename                                   |
| Alt+Z      | Refactor (context menu)                  |

## Bookmark

| Key                 | Action                       |
| ------------------- | ---------------------------- |
| Ctrl+{1 to 9}       | Go to bookmark               |
| Ctrl+Shift+{1 to 9} | Set bookmark on current line |

## Folding

| Key             | Action                      |
| --------------- | --------------------------- |
| Alt+{[,]}       | Collapse/expand             |
| Alt+Shift+{[,]} | Collapse/expand recursively |
| Alt+Ctrl+{[,]}  | Collapse/expand all         |

## Autocomplete / Code-gen

| Key              | Action             |
| ---------------- | ------------------ |
| Ctrl+Spacebar    | Basic completion   |
| Ctrl+Shift+Enter | Complete statement |
| Alt+Insert       | New file           |
| Alt+Shift+Insert | New folder         |
| `<none>`         | Implement methods  |
| `<none>`         | Surround with...   |

## AI

| Key                 | Action                                              |
| ------------------- | --------------------------------------------------- |
| Ctrl+Shift+Spacebar | Autocomplete code with AI                           |
| Ctrl+J              | Generate code with AI at caret                      |
| Tab                 | Accept autocomplete suggestion                      |
| Ctrl+Right          | Accept part of the autocomplete suggestion          |
| Ctrl+Shift+Right    | Accept a single line of the autocomplete suggestion |
| Alt+J               | AI Assistant chat                                   |
| Ctrl+H              | Chat History (not working in JetBrains)             |
| Alt+K               | AI agentic helper                                   |
| `<none>`            | Explain code                                        |
| `<none>`            | Generate documentation                              |
| `<none>`            | Suggest refactorings                                |
| `<none>`            | Find problems                                       |
| `<none>`            | AI Chat with selection                              |

## Tool windows

| Key         | Action                                                               |
| ----------- | -------------------------------------------------------------------- |
| Alt+`       | Terminal (focus existing, or open new if none)                       |
| Alt+Shift+` | New terminal (always opens a new tab)                                |
| Alt+1       | Project explorer (files / view / structure) - Highlights active file |
| Alt+2       | VCS log window                                                       |
| Alt+S       | Agents panel (AI / Workspaces)                                       |
| Alt+Shift+S | Agent chat window                                                    |

## Run

| Key            | Action                             |
| -------------- | ---------------------------------- |
| Alt+R, R       | Run (current/last configuration)   |
| Alt+R, D       | Debug (current/last configuration) |
| Alt+R, Shift+R | Run (choose configuration)         |
| Alt+R, Shift+D | Debug (choose configuration)       |
| Alt+R, F       | Run current file                   |
| Alt+R, B       | Build current file                 |
| Alt+R, E       | Edit run configurations            |
| Alt+R, S       | Stop current run                   |
| Alt+R, H       | Run/test history                   |

## Language specific

| Key      | Action                    |
| -------- | ------------------------- |
| `<none>` | Macro expansion           |
| `<none>` | Recursive macro expansion |
| `<none>` | Full file macro expansion |
| `<none>` | REPL                      |
