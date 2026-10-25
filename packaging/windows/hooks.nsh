; The installer runs elevated (perMachine), so it can manage the service directly.

!macro NSIS_HOOK_PREINSTALL
  ; An upgrade replaces blkbstrd.exe, which Windows will not do while the service is running it.
  nsExec::Exec 'sc.exe stop blkbstrd'
  Sleep 3000
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Fails quietly on a first install: the service is registered later, from the app.
  nsExec::Exec 'sc.exe start blkbstrd'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; An app update runs the old uninstaller first, with /UPDATE. Removing the service there would
  ; leave the user with no helper (and no engine exclusion) after every update.
  ${If} $UpdateMode = 1
    nsExec::Exec 'sc.exe stop blkbstrd'
    Sleep 3000
  ${Else}
    nsExec::Exec '"$INSTDIR\blkbstrd.exe" uninstall-service'
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ReadEnvStr $0 ProgramData
    RMDir /r "$0\blkbstr"
  ${EndIf}
!macroend
