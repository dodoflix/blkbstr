# Windows packaging

| File | What it is |
| --- | --- |
| `../../src-tauri/tauri.windows.conf.json` | Makes the Tauri bundle a per-machine NSIS installer that carries `blkbstrd.exe` |
| `hooks.nsh` | Installer hooks: stop the service before an upgrade, remove it on uninstall |
| `stage-daemon.ps1` | Builds `blkbstrd` and puts it where the bundle expects it. Run before any build of the GUI crate |
| `sign.ps1` | Signs one file with the certificate in the `WINDOWS_CERT_PFX_BASE64` secret |
| `render.sh` | Fills `winget/` and `chocolatey/` for one release |
| `winget/`, `chocolatey/` | Package templates with `__VERSION__`, `__URL__` and `__SHA256__` placeholders |

The release workflow runs all of this and attaches the filled-in packages as `windows-packages.zip`.
Secrets and the manual steps before a first release are in [docs/releasing.md](../../docs/releasing.md).

Signing is **temporarily** off: with no certificate the installer is unsigned, and `sign.ps1` is
only called when the secrets exist.

## Publishing

**Temporary:** nothing here publishes to a package repository; both need an account only a
maintainer has, and there is none yet. Submit by hand until the release workflow can do it.

**winget.** The first version is a pull request to
[microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) with the three files from
`winget/` under `manifests/b/Blkbstr/Blockbuster/<version>/`. Check them with
`winget validate --manifest <dir>` first. Later versions can use `wingetcreate update`.

**Chocolatey.** From `chocolatey/`: `choco pack`, then `choco push blkbstr.<version>.nupkg
--source https://push.chocolatey.org/ --api-key <key>`. The community repository moderates every
version, and will ask for the antivirus scan results.
