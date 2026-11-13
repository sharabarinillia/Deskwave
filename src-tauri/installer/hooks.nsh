; Uninstall clean-up for the setup.exe installer.
; Tauri's uninstaller already removes the "Deskwave" startup entry. Its "Delete app data" box only
; covers the folder named after the bundle id, but Deskwave keeps its data in %APPDATA%\Deskwave.
; Remove Deskwave's own settings and widget data there. The mods and wallpapers folders hold files
; the user put there (their own videos, downloaded mods), so those are never deleted.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    Delete "$APPDATA\Deskwave\config.json"
    RmDir /r "$APPDATA\Deskwave\storage"
    ; Only succeeds if the folder is now empty.
    RmDir "$APPDATA\Deskwave"
  ${EndIf}
!macroend
