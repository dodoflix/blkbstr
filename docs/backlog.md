# Backlog

Every task, grouped by area and assigned a milestone from [plan.md](plan.md). Checked items are
done in this repository.

## Onboarding & installation

- [x] Reachability check against a list of commonly blocked sites, with a per-site verdict - M2
- [x] Locate the engine binary; refuse by name when missing - M1
- [x] Report the installed zapret2 version - M1
- [x] Check for LuaJIT 2.1+ / Lua 5.3+ before anything else - M2
- [x] Engine installation on Linux: the pinned upstream release, hash-checked, installed after a
      polkit prompt - M2
- [x] Engine installation on Windows - M3
- [x] Linux distro detection from `ID`/`ID_LIKE`; shown in advanced mode, no longer needed to
      install the engine - M2
- [x] First-run wizard: detect, check, auto-configure, save, re-runnable later - M2
- [x] Detect an existing zapret2 install - M2
- [x] Back up the existing install's config somewhere the user is told about, import it as a
      config named `Legacy` - M2

## Configuration management

- [x] Portable config format: strategies as nfqws2 profiles, ordered Lua actions - M0
- [x] Lua function registry; unknown functions warn and pass through - M0
- [x] `compat` tracks `NFQWS2_COMPAT_VER`; a mismatch warns - M1
- [x] Values validated against option injection into the parameter file - M1
- [x] Parameter-file preview, so the UI can show what will actually run - M1
- [x] Save, list and load named configs in the user's own directory - M0
- [ ] Config editor UI with live linting - unscheduled
- [x] Ephemeral "try it" mode with confirm-or-revert timeout - M2
- [x] Auto-configuration: rank candidate strategies, apply each ephemerally, re-test, keep the
      first that works, save under a user-chosen name - M2. Built on our own reachability check
      rather than wrapping `blockcheck2.sh`: the walk needs a fast structured oracle it can call
      between candidates, which an interactive shell script is not
- [ ] Optional sync to a user-supplied GitHub repo - M4
- [ ] Client-side encryption for synced configs - M4

## Service & privilege management

- [x] Privilege split: unprivileged GUI, privileged daemon, local socket - M0
- [x] Socket permissions as the authorisation model (`0660`, `blkbstr` group) - M0
- [x] Linux systemd unit with a restricted capability set - M0
- [x] Linux polkit action for enabling/disabling the service - M0
- [x] Linux install and uninstall scripts - M0
- [x] Windows Service, installed by one UAC-elevated step - M3
- [x] Windows named-pipe ACL restricted to the installing user - M3
- [ ] BSD rc.d service script - M5
- [x] Service control from the GUI: enable/disable at boot, start/stop now - M1

## Engine

- [x] Render a `Config` to an nfqws2 parameter file, passed as `@file` - M1
- [x] Validate with the engine's own `--intercept=0` before touching the network stack - M1
- [x] nftables interception in a dedicated table; teardown is one atomic delete - M1
- [x] Intercept only the ports the enabled strategies select - M1
- [x] Notice an engine that exited by itself and surface the reason - M1
- [x] Restart a crashed engine with backoff; give up after a crash loop - M1
- [x] Teardown on `Drop` and on next startup, not only on clean stop - M1
- [x] Real `EngineStatus`: pid, started_at, engine version, last error - M1
- [x] Surface uptime, pid and engine version in the GUI - M1
- [x] Start on boot with the last non-ephemeral config - M1
- [x] `winws2` backend for the same `Config` - M3
- [ ] `dvtws2` backend for FreeBSD and OpenBSD - M5

## Logging & diagnostics

- [x] Rolling daily log files, plus stderr for the service manager - M0
- [x] Log viewer in the GUI, tailing the file live - M1
- [x] Filter within logs - M1
- [x] One-click diagnostic export for bug reports - M1

## Auto-update

- [ ] App updates via Tauri's updater plugin against GitHub Releases - M4
- [ ] Engine updates, tracked separately from app updates - M4

## Appearance & UX

- [x] Follows the OS light/dark setting - M0
- [x] Explicit light/dark override, persisted - M4
- [ ] System tray / menu bar toggle with a status indicator - M4
- [ ] Localisation - M4
- [ ] Accessibility pass: keyboard navigation, screen reader labels, contrast - M4

## Security & privacy

- [x] GUI never runs elevated - M0
- [x] Config names validated before reaching the filesystem - M0
- [x] No telemetry, of any kind, opt-in or otherwise - M0
- [ ] Warn before enabling sync that a config reveals which sites the user visits - M4
- [ ] Sign all binaries and installers - M3. The pipeline is wired and waits on a certificate; releases are unsigned until then
- [x] Publish checksums and a verification procedure - M3

## Platform friction

- [x] AppArmor and SELinux profiles for the Linux daemon - M1
- [ ] Non-systemd init support - deferred until requested
- [ ] iptables fallback for kernels older than 5.15 - deferred until requested
- [x] AV false-positive submission process, documented in [av-submission.md](av-submission.md) - M3
- [ ] Keep the Lua function registry in step with upstream releases - ongoing

## Packaging & distribution

- [ ] Flatpak, AUR, deb repo, AppImage - ongoing
- [x] winget, Chocolatey packages built per release - M3. Published by hand until the accounts exist
- [ ] FreeBSD and OpenBSD ports - M5

## Project & process

- [x] CI: portable crates and frontend in a fast job, GUI build on Linux and Windows - M0
- [x] MIT licence, matching upstream zapret2 - M0
- [x] Contribution guidelines - M0
- [x] Code of conduct - M0
- [x] Issue and PR templates - M0
- [x] Release process: tag-driven build, checksums, provenance attestation - M1
