# AutoHotkey Scripts

Windows automation scripts for AutoHotkey 2.0.

## Scripts

| Script                       | Purpose                                                      |
| ---------------------------- | ------------------------------------------------------------ |
| `load_all.ahk`               | Discovers and runs all other `.ahk` scripts in the directory |
| `dim_secondary_monitors.ahk` | Dims secondary monitors                                      |
| `hdr_toggle.ahk`             | Toggle HDR display mode                                      |
| `run_or_activate.ahk`        | Launch an app or focus its window if already running         |
| `show_running_scripts.ahk`   | Show a list of currently running AHK scripts                 |
| `zed_numpad_remap.ahk`       | Remap Ctrl/Alt+Numpad to F-keys while Zed is active          |

## Setup

Run `load_all.ahk` on startup (e.g. via the Windows Shortcuts in
`home/windows_shortcuts/`). It will load all other scripts automatically.
