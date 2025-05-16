#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon

#`:: RunOrFocus("wezterm-gui.exe", "ahk_exe wezterm-gui.exe")
#1:: RunOrFocus("C:\Users\hesos", "File Explorer")
#2:: RunOrFocus("rustrover64.exe", "ahk_exe rustrover64.exe")

; Runs the given `target` if the given `window_title` doesn't exist. Otherwise,
; activates the given window, or minimizes it if it's currently active.
RunOrFocus(target, window_title) {
    if WinActive(window_title) {
        WinMinimize(window_title)
    } else if WinExist(window_title) {
        WinActivate(window_title)
    } else {
        try {
            Run(target)
        } catch Error as e {
            MsgBox("Error running: " . target . "`n" . e.Message, "Program Launch Error", "Icon!")
        }
    }
}
