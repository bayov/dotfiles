#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon

#`:: run_or_focus("wezterm-gui.exe", "ahk_exe wezterm-gui.exe")
#1:: run_or_focus("C:\Programs\VS Code\Code.exe", "ahk_exe Code.exe")  ; VS Code
#2:: run_or_focus_file_explorer()

; Runs the given `target` if the given `window_title` doesn't exist. Otherwise,
; activates the given window, or minimizes it if it's currently active.
run_or_focus(target, window_title) {
    if WinActive(window_title) {
        WinMinimize(window_title)
    } else if WinExist(window_title) {
        WinActivate(window_title)
    } else {
        try {
            Run(target)
            ; After running, wait at most 0.5 seconds and activate window
            if WinWait(window_title, , 0.5)
                WinActivate(window_title)
        } catch Error as e {
            MsgBox("Error running: " . target . "`n" . e.Message, "Program Launch Error", "Icon!")
        }
    }
}

; For file explorer, we also bring all existing windows to the front
run_or_focus_file_explorer() {
    ; If the current active window is already a file explorer, minimize all
    ; file explorer windows
    if WinActive("ahk_class CabinetWClass") {
        for hwnd in WinGetList("ahk_class CabinetWClass") {
            WinMinimize(hwnd)
        }
        return
    }

    ; Otherwise, if a file explorer window already exists, bring to front all
    ; file explorer windows
    if WinExist("ahk_class CabinetWClass") {
        ; If it exists, loop through all File Explorer windows and bring them to
        ; the front.
        for hwnd in WinGetList("ahk_class CabinetWClass") {
            WinActivate(hwnd)
        }
        return
    }

    ; And lastly, if no file explorer windows exist, open one
    Run("C:\Users\hesos")
}
