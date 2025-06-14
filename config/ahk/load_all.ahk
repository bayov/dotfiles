#Requires AutoHotkey v2.0
#SingleInstance Force
#NoTrayIcon

script_folder := ".\"  ; Same folder as this script

; Check if the script_folder exists
if not DirExist(script_folder) {
    MsgBox("Error: Folder does not exist: " . script_folder, "Scripts folder not found", "Icon!")
    ExitApp
}

RunScript(path) {
    try {
        OutputDebug("Running script: " . path)
        Run(path)
    } catch Error as e {
        MsgBox("Error running script: " . path . "`n`nDetails: " . e.Message, "Script Execution Error", "Icon!")
    }
}

loop files, script_folder . "\*.ahk" {
    script := A_LoopFileName
    if (script = A_ScriptName) {
        continue  ; skip this script
    }
    RunScript(script)
}

; Keybind to reload everything (we just reload this script, which reloads every
; other script).
#-:: {
    MsgBox("Reloading scripts")
    RunScript(A_ScriptFullPath)
}
