#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon

DetectHiddenWindows(True)

#0:: {
    hwnds_list := WinGetList("ahk_class AutoHotkey")
    running_scripts := ""
    for hwnd in hwnds_list {
        title := WinGetTitle("ahk_id " . hwnd)
        title := RegExReplace(title, " - AutoHotkey v[\.0-9]+$")
        ; Ignore launcher.ahk instances
        if RegExMatch(title, "AutoHotkey\\UX\\launcher.ahk") {
            continue
        }
        running_scripts .= (running_scripts ? "`r`n" : "") . title
    }
    MsgBox(running_scripts, "Running AHK scripts")
}
