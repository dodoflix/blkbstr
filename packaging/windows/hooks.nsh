; The installer runs elevated (perMachine), so it can manage the service directly.

; Stops the service and waits, up to 15 seconds, until it is neither running nor stopping. Windows
; will not replace blkbstrd.exe while the service has it open. Get-Service is used because
; `sc query` prints text that is translated.
!macro BlkbstrStopService
  Push $0
  Push $1
  nsExec::Exec 'sc.exe stop blkbstrd'
  Pop $0
  StrCpy $1 0
  ${Do}
    nsExec::Exec `powershell.exe -NoProfile -NonInteractive -Command "exit [int]((Get-Service blkbstrd -ErrorAction SilentlyContinue).Status -in 'Running','StopPending')"`
    Pop $0
    ${If} $0 = 0
      ${ExitDo}
    ${EndIf}
    IntOp $1 $1 + 1
    ${If} $1 >= 30
      ${ExitDo}
    ${EndIf}
    Sleep 500
  ${Loop}
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro BlkbstrStopService
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Fails quietly on a first install: the service is registered later, from the app.
  nsExec::Exec 'sc.exe start blkbstrd'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; An app update runs the old uninstaller first, with /UPDATE. Removing the service there would
  ; leave the user with no helper (and no engine exclusion) after every update.
  ${If} $UpdateMode = 1
    !insertmacro BlkbstrStopService
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
