#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon

; Remap Numpad keys to F-keys while the Zed editor is the active window.
;
; Only fires when a Ctrl and/or Alt modifier is held (Ctrl, Alt, or Ctrl+Alt).
; Plain numpad presses are left untouched, so the numpad still types numbers
; inside Zed. The held modifier passes through via {Blind}, so Ctrl+Numpad1
; produces Ctrl+F1, Alt+Numpad1 produces Alt+F1, and so on.
;
; Mapping:
;   Numpad 1..9 -> F1..F9      Numpad /  -> F12     Numpad -     -> F16
;   Numpad 0    -> F10         Numpad *  -> F13     Numpad +     -> F17
;   Numpad .    -> F11                              Numpad Enter -> F18
;
; F14 and F15 are deliberately skipped (Numpad - jumps from F13 to F16). On
; macOS those two F-keys are practically impossible to bind: The OS reserves
; them and most keymap editors refuse to record them. This keymap is shared
; across machines, so it avoids F14/F15 entirely.

; Numpad key -> target F-key. Each numpad number has two AHK names depending on
; NumLock state (e.g. Numpad1 vs NumpadEnd); both are mapped so the remap works
; regardless of whether NumLock is on.
remap := Map(
    "Numpad0",     "F10",  "NumpadIns",   "F10",
    "Numpad1",     "F1",   "NumpadEnd",   "F1",
    "Numpad2",     "F2",   "NumpadDown",  "F2",
    "Numpad3",     "F3",   "NumpadPgDn",  "F3",
    "Numpad4",     "F4",   "NumpadLeft",  "F4",
    "Numpad5",     "F5",   "NumpadClear", "F5",
    "Numpad6",     "F6",   "NumpadRight", "F6",
    "Numpad7",     "F7",   "NumpadHome",  "F7",
    "Numpad8",     "F8",   "NumpadUp",    "F8",
    "Numpad9",     "F9",   "NumpadPgUp",  "F9",
    "NumpadDot",   "F11",  "NumpadDel",   "F11",
    "NumpadDiv",   "F12",
    "NumpadMult",  "F13",
    "NumpadSub",   "F16",
    "NumpadAdd",   "F17",
    "NumpadEnter", "F18")

; Scope every hotkey registered below to the Zed window only.
HotIf(zed_active)
for numpad_key, f_key in remap {
    for modifier in ["^", "!", "^!"] {  ; Ctrl, Alt, Ctrl+Alt
        Hotkey(modifier . numpad_key, send_key(f_key))
    }
}
HotIf()  ; reset context for any later hotkeys

zed_active(*) => WinActive("ahk_exe Zed.exe")

; Returns a hotkey handler that sends `f_key`. {Blind} keeps the held Ctrl/Alt
; pressed, so the modifier is preserved on the F-key (Ctrl+Numpad1 -> Ctrl+F1).
send_key(f_key) => (*) => Send("{Blind}{" . f_key . "}")
