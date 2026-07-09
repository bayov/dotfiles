#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon
#MaxThreadsPerHotkey 2

;; Track the current state: 0 = Off, 1 = F10 running, 2 = F11 running
;global ActiveMacro := 0

;; ----------------------------------------------------------------------
;; The following hotkeys will ONLY work when WoW Classic is active
;; ----------------------------------------------------------------------
;#HotIf WinActive("ahk_exe WowClassic.exe")

;; Trigger 1: F10
;$F10:: {
;    global ActiveMacro
;    if (ActiveMacro == 1) {
;        ; STOP STATE
;        ActiveMacro := 0
;        SetTimer(SpamF10, 0)
;        Send("{F12}") ; Sends F12 as it shuts down
;    } else {
;        ; START STATE
;        ActiveMacro := 1
;        SetTimer(SpamF11, 0)
;        SpamF10()
;    }
;}

;; Trigger 2: F11
;$F11:: {
;    global ActiveMacro
;    if (ActiveMacro == 2) {
;        ; STOP STATE
;        ActiveMacro := 0
;        SetTimer(SpamF11, 0)
;        Send("{F12}") ; Sends F12 as it shuts down
;    } else {
;        ; START STATE
;        ActiveMacro := 2
;        SetTimer(SpamF10, 0)
;        SpamF11()
;    }
;}

;; Trigger 3: F12 (Dedicated Stop Button)
;$F12:: {
;    global ActiveMacro
;    if (ActiveMacro != 0) {
;        ; If either macro is running, turn them off
;        ActiveMacro := 0
;        SetTimer(SpamF10, 0)
;        SetTimer(SpamF11, 0)
;    }
;    ; This fires unconditionally, whether a macro was running or not
;    Send("{F12}")
;}

;#HotIf
;; ----------------------------------------------------------------------
;; End of WoW-specific hotkeys
;; ----------------------------------------------------------------------

;; The randomized action for F10
;SpamF10() {
;    global ActiveMacro
;    if (ActiveMacro == 1) {
;        if WinActive("ahk_exe WowClassic.exe") {
;            Send("{F10}")
;            SetTimer(SpamF10, -GetNormalDelay())
;        } else {
;            ; If alt-tabbed, don't click. Wait 250ms and check again.
;            SetTimer(SpamF10, -250)
;        }
;    }
;}

;; The randomized action for F11
;SpamF11() {
;    global ActiveMacro
;    if (ActiveMacro == 2) {
;        if WinActive("ahk_exe WowClassic.exe") {
;            Send("{F11}")
;            SetTimer(SpamF11, -GetNormalDelay())
;        } else {
;            ; If alt-tabbed, don't click. Wait 250ms and check again.
;            SetTimer(SpamF11, -250)
;        }
;    }
;}

;; ----------------------------------------------------------------------
;; Math Function to generate a Normal Distribution (Bell Curve)
;; ----------------------------------------------------------------------
;GetNormalDelay() {
;    mu := 278
;    sigma := 44

;    loop {
;        u1 := Random(1, 1000000) / 1000000.0
;        u2 := Random(1, 1000000) / 1000000.0
;        z0 := Sqrt(-2.0 * Ln(u1)) * Cos(2.0 * 3.141592653589793 * u2)
;        val := Round(mu + (z0 * sigma))

;        if (val >= 142 && val <= 411) {
;            return val
;        }
;    }
;}
