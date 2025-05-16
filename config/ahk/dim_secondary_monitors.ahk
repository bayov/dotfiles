#NoEnv                   ; Recommended for performance and compatibility with future AutoHotkey releases
#SingleInstance, Force   ; Ensure only one instance of the script runs
#MaxThreadsPerHotkey, 2  ; Allow hotkeys to run multiple instances if pressed repeatedly
#NoTrayIcon              ; Hide tray icon (remove/comment out if you want to see it)

;===================================
; Monitor Configuration Overrides
;===================================
; Set these override values to 0 to use automatic detection
; For secondary monitors, overrides are set per monitor index

; Primary Monitor Override
Primary_Width_Override := 0      ; Set to 0 for automatic detection
Primary_Height_Override := 0     ; Set to 0 for automatic detection

; Secondary Monitors Overrides
Secondary_Width_Override := {}
Secondary_Height_Override := {}

; Example Overrides:
; To override the width and height of secondary monitor 2:
; Secondary_Width_Override[2] := 1920
; Secondary_Height_Override[2] := 1080

Secondary_Width_Override[1] := 1440
Secondary_Height_Override[1] := 2560

;===================================
; Optional Initialization Delay
;===================================
; Introduce a 10-second delay only if the system has been up for less than 60 seconds
if (A_TickCount < 60000) {
    Sleep, 10000
}

CoordMode, Pixel, Screen
CoordMode, Mouse, Screen

;===================================
; Global Variables Initialization
;===================================
HwndPrimary := 0           ; Window handle for the primary (main) monitor
HwndSecondary := []        ; Array to store window handles for secondary monitors
ToggleSecondary := false   ; Initialize toggle state for secondary monitors as visible
TogglePrimary := false     ; Initialize toggle state for the primary monitor as visible

;===================================
; Initialization Code
;===================================
; Retrieve Monitor Information
SysGet, MonitorCount, MonitorCount
SysGet, PrimaryMonitorIndex, MonitorPrimary

; Iterate through each monitor to create overlay GUIs
Loop, %MonitorCount%
{
    CurrentMonitorIndex := A_Index
    SysGet, Monitor, Monitor, %CurrentMonitorIndex%
    ; MonitorLeft, MonitorTop, MonitorRight, MonitorBottom are automatically set by SysGet
    Monitor_Left := MonitorLeft
    Monitor_Top := MonitorTop
    Monitor_Right := MonitorRight
    Monitor_Bottom := MonitorBottom
    Width := Monitor_Right - Monitor_Left
    Height := Monitor_Bottom - Monitor_Top

    ;===================================
    ; Apply Width and Height Overrides
    ;===================================
    if (CurrentMonitorIndex = PrimaryMonitorIndex) {
        OverrideWidth := Primary_Width_Override
        OverrideHeight := Primary_Height_Override
    } else {
        ; For secondary monitors, check if an override exists for this specific monitor
        if (Secondary_Width_Override.HasKey(CurrentMonitorIndex))
            OverrideWidth := Secondary_Width_Override[CurrentMonitorIndex]
        else
            OverrideWidth := 0  ; No override, use automatic detection

        if (Secondary_Height_Override.HasKey(CurrentMonitorIndex))
            OverrideHeight := Secondary_Height_Override[CurrentMonitorIndex]
        else
            OverrideHeight := 0  ; No override, use automatic detection
    }

    if (OverrideWidth != 0)
        Width := OverrideWidth  ; Override width if specified

    if (OverrideHeight != 0)
        Height := OverrideHeight  ; Override height if specified

    ;===================================
    ; Debugging: Ensure Width and Height Are Valid
    ;===================================
    if (Width <= 0 || Height <= 0) {
        MsgBox, 16, Error, Invalid Width or Height detected for Monitor %CurrentMonitorIndex%.`nWidth: %Width%`nHeight: %Height%
        ExitApp  ; Terminate the script to prevent further errors
    }

    ;===================================
    ; Create Overlay GUI Window
    ;===================================
    Gui, New, +AlwaysOnTop -Caption +ToolWindow +Owner +E0x20 +LastFound
    Gui, Color, Black
	
    ; Adding a Text control that covers the entire GUI to capture click events
    Gui, Add, Text, x0 y0 w%Width% h%Height% gHandleClick,

    Gui, Show, Hide x%Monitor_Left% y%Monitor_Top% w%Width% h%Height%
    WinSet, Transparent, 255, A  ; Ensure the GUI is fully opaque

    ; Retrieve the window handle
    hWnd := WinExist()

    ; Assign the window handle to primary or secondary monitor
    if (CurrentMonitorIndex = PrimaryMonitorIndex) {
        HwndPrimary := hWnd
    } else {
        HwndSecondary.Push(hWnd)
    }
}

return  ; End of auto-execute section

;===================================
; Hotkey: Win + PgUp to Toggle Secondary Monitors
;===================================
#PgUp::
    global HwndSecondary, ToggleSecondary
    ToggleSecondary := !ToggleSecondary  ; Toggle the state

    for index, hwnd in HwndSecondary
    {
        if (ToggleSecondary) {
            WinShow, ahk_id %hwnd%
        } else {
            WinHide, ahk_id %hwnd%
        }
    }
return

;===================================
; Hotkey: Win + PgDn to Toggle Primary Monitor
;===================================
#PgDn::
    global HwndPrimary, TogglePrimary
    TogglePrimary := !TogglePrimary  ; Toggle the state

    if (TogglePrimary) {
        WinShow, ahk_id %HwndPrimary%
    } else {
        WinHide, ahk_id %HwndPrimary%
    }
return

;===================================
; GUI Click Handler to Close the Window with Dark Gray Flash Indication
; and Update Toggle States
;===================================
HandleClick:
    ; Declare global variables to access toggle states and window handles
    global TogglePrimary, ToggleSecondary, HwndPrimary, HwndSecondary

    ; Get the HWND of the active window (the GUI that was clicked)
    WinGet, clickedHwnd, ID, A

    ; ===============================
    ; *** Modified Flash Color ***
    ; ===============================
    ; Flash the GUI to dark gray to indicate closing
    Gui, Color, 222222, ahk_id %clickedHwnd%
    WinSet, Redraw,, ahk_id %clickedHwnd%

    ; Wait briefly to show the flash	
    Sleep, 200  ; Duration in milliseconds

    ; Restore the original color (Black)
    Gui, Color, Black, ahk_id %clickedHwnd%
    WinSet, Redraw,, ahk_id %clickedHwnd%

    ; Hide the GUI window
    WinHide, ahk_id %clickedHwnd%

    ;===================================
    ; Update Toggle States Based on Closed Window
    ;===================================
    if (clickedHwnd = HwndPrimary) {
        ; If the primary window was closed, update its toggle state
        TogglePrimary := false
    }
    else {
        ; Check if the clicked window is in the secondary windows list
        found := false
        for index, hwnd in HwndSecondary
        {
            if (hwnd = clickedHwnd)
            {
                found := true
                break
            }
        }

        if (found) {
            ; If a secondary window was closed, update its toggle state
            ToggleSecondary := false
        }
    }
return
