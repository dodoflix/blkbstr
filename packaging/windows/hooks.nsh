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
  nsExec::Exec '"$INSTDIR\blkbstrd.exe" uninstall-service'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ReadEnvStr $0 ProgramData
  RMDir /r "$0\blkbstr"
!macroend
