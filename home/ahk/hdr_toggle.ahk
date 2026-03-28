#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon

hdr_cmd_path := "C:\Programs\HDRTray\HDRCmd.exe"

#h:: {
    ; Create a temporary GUI that looks like a MsgBox
    gui_obj := Gui("+AlwaysOnTop -MaximizeBox -MinimizeBox -SysMenu +Border +ToolWindow", "HDR Control")
    gui_obj.SetFont("s10")

    ; Get current HDR status as text (e.g., "HDR is on", "HDR is off")
    exit_code := RunWait(hdr_cmd_path . " status -m exitcode",, "Hide")
    hdr_status := ""
    if exit_code = 0 {
        hdr_status := "HDR is on"
    } else if exit_code = 1 {
        hdr_status := "HDR is off"
    } else {
        hdr_status := "Error: Invalid HDR status"
    }

    gui_obj.Add("Text", "ReadOnly Center w250", hdr_status)
    on_btn := gui_obj.Add("Button", "Center w250", "Turn HDR on")
    off_btn := gui_obj.Add("Button", "Center w250", "Turn HDR off")
    close_btn := gui_obj.Add("Button", "Center w250", "Close")

    on_btn.OnEvent("Click", enable_hdr)
    off_btn.OnEvent("Click", disable_hdr)
    close_btn.OnEvent("Click", (*) => gui_obj.Destroy())

    ; Ensure GUI closes properly
    gui_obj.OnEvent("Close", (*) => gui_obj.Destroy())
    gui_obj.Show("Center") ; Show the GUI in the center of the screen
}

; --- Functions for GUI buttons ---

enable_hdr(btn, _info) {
    global hdr_cmd_path
    Run(hdr_cmd_path . " on",, "Hide")
    btn.Gui.Destroy()
}

disable_hdr(btn, _info) {
    global hdr_cmd_path
    Run(hdr_cmd_path . " off",, "Hide")
    btn.Gui.Destroy()
}
