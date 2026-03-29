#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon
#MaxThreadsPerHotkey 2

;===================================
; Configuration & Constants
;===================================
OPENRGB_PATH := "C:\Programs\OpenRGB\OpenRGB.exe"
PROFILE_ON   := "Bayov"
PROFILE_OFF  := "Off"

;===================================
; Optional Initialization Delay
;===================================
; Introduce a 10-second delay only if the system has been up for less than 60 seconds
if A_TickCount < 60000 {
    Sleep(10000)
}

; Since this program covers the entire monitor with a black window to dim it, we
; need the GUI elements to be per-monitor DPI aware.
;
; See https://www.autohotkey.com/docs/v2/misc/DPIScaling.htm for more details.
DllCall("SetThreadDpiAwarenessContext", "ptr", -3, "ptr")

;===================================
; Global Variables Initialization
;===================================
; Array of Gui objects by monitor index
gui_by_monitor_index := []

; Array of dimming state by monitor index, where `true` means dimmed.
dim_by_monitor_index := []
rgb_enabled := true

;===================================
; Monitor Setup
;===================================
; Retrieve Monitor Information
monitor_counter := MonitorGetCount()
primary_monitor_index := MonitorGetPrimary()

; Iterate through each monitor to create overlay GUIs
Loop monitor_counter {
    monitor_index := A_Index
    MonitorGet(monitor_index, &left, &top, &right, &bottom)
    width := right - left
    height := bottom - top

    gui_obj := Gui("-DPIScale +AlwaysOnTop -Caption +Owner +ToolWindow", "Dim")

    ; We can't register click on Gui object, only on GuiControl, so we create
    ; an invisible empty text object inside the Gui.
    gui_control := gui_obj.AddText("x0 y0 w" . width . " h" . height)
    gui_control.OnEvent("Click", on_click_close)

    gui_obj.BackColor := "Black"
    gui_obj.Show("Hide x" . left . " y" . top . " w" . width . " h" . height)

    gui_obj.DefineProp("monitor_index", {Value: monitor_index})
    gui_by_monitor_index.Push(gui_obj)
    dim_by_monitor_index.Push(false)
}

toggle_monitor_dimming(monitor_index, flash := false) {
    global gui_by_monitor_index, dim_by_monitor_index

    dim_by_monitor_index[monitor_index] := !dim_by_monitor_index[monitor_index]
    local dim := dim_by_monitor_index[monitor_index]

    local gui_obj := gui_by_monitor_index[monitor_index]

    if dim {
        gui_obj.Show("NA")  ; Show window without stealing focus (NoActivate)
    } else {
        if flash {
            gui_obj.BackColor := "222222"
            Sleep(200)
        }
        gui_obj.Hide()
        if flash {
            gui_obj.BackColor := "Black"
        }
    }
}

on_click_close(gui_control, _info) {
    toggle_monitor_dimming(gui_control.Gui.monitor_index, true)
}

;===================================
; Hotkeys
;===================================

; Dim Secondary Monitors
#[:: {
    global monitor_counter
    Loop monitor_counter {
        local monitor_index := A_Index
        if monitor_index = primary_monitor_index {
            continue
        }
        toggle_monitor_dimming(monitor_index)
    }
}

; Dim Primary Monitor
#]:: toggle_monitor_dimming(primary_monitor_index)

; Toggle OpenRGB Profile
#\:: {
    global rgb_enabled
    rgb_enabled := !rgb_enabled

    target_profile := rgb_enabled ? PROFILE_ON : PROFILE_OFF

    ; Calls the executable with the profile flag.
    ; Since your server is running, this acts as a client command.
    Run(OPENRGB_PATH . " --profile " . target_profile, , "Hide")
}