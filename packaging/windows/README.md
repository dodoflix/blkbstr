# Windows packaging

| File | What it is |
| --- | --- |
| `../../src-tauri/tauri.windows.conf.json` | Makes the Tauri bundle a per-machine NSIS installer that carries `blkbstrd.exe` |
| `hooks.nsh` | Installer hooks: stop the service before an upgrade (it stays registered), remove it on uninstall |
| `stage-daemon.ps1` | Builds `blkbstrd` and puts it where the bundle expects it. Run before any build of the GUI crate |

The release workflow runs `stage-daemon.ps1`, builds the installer and attaches it to the release.
It is not signed and not published to winget or Chocolatey.
