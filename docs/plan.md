# Plan

## What this is

Blockbuster (`blkbstr`) is a desktop GUI for [zapret2](https://github.com/bol-van/zapret2): it
installs the engine, generates and manages configurations, runs it as a background service, and
shows what it is doing — on Linux, Windows and the BSDs.

zapret2 describes itself as *"a tool for enthusiasts, not a ready-made solution for beginners"*.
Its own documentation is a 5,000-line manual, its strategies are Lua programs, and a working
setup is a parameter file full of lines like
`--lua-desync=fake:blob=fake_default_tls:badsum:strategy=1`. That is fine for the people who wrote
it and a wall for everyone else — which matters, because the people most in need of it are rarely
the people most comfortable with that string. Blockbuster's job is to make a working configuration
reachable without the user ever seeing it, while never hiding it from someone who wants it.

## Non-goals

- Not a fork of zapret2. Blockbuster drives the upstream engine; the Lua strategy libraries stay
  upstream, and we do not ship our own attacks.
- Not a VPN, proxy, or traffic router. It manipulates the local packet path only.
- Not a mobile app. Desktop only; the Android/iOS network-extension story is a different project.
- Not a macOS app. Upstream does not support macOS and does not expect to — Apple removed
  `ipdivert` from the kernel and there is no replacement interception facility.
- No account, no server, no telemetry. Optional GitHub sync is the user's own repo.

## Success criteria

A first-time user on any supported OS goes from "sites are blocked" to "sites load, and it still
works after a reboot" without opening a terminal, and can hand the resulting config to a friend on
a different OS.

## Architecture in one paragraph

An unprivileged Tauri app (`blkbstr`) talks over a local socket to a privileged service
(`blkbstrd`) that owns the engine process and the firewall rules. Configs are portable JSON owned
by the user; the daemon renders them into an nfqws2 parameter file, validates it with the engine's
own `--dry-run`, installs the interception rules and supervises the process. Details in
[architecture.md](architecture.md).

## Milestones

Each milestone is shippable — a release with a working feature — not a layer of an unfinished
cake. Numbering is order, not dates.

### M0 — Skeleton *(done)*

The repository this document lives in.

- Cargo workspace: `blkbstr-core` (config model, function registry, renderer, protocol),
  `blkbstr-daemon` (socket server), `src-tauri` (GUI backend, daemon client, config file CRUD).
- React + Radix Themes shell with a status panel and a config list, following the OS theme.
- Linux packaging: systemd unit with a restricted capability set, polkit action, install and
  uninstall scripts.
- CI: build and test on all three OSes.

Everything except the engine itself: validation, linting, transport, privilege boundary and
packaging, so M1 fills a hole rather than inventing a shape.

### M1 — It actually works on Linux

The first release that does the thing.

- [x] Render a `Config` to an nfqws2 parameter file, consumed via `@file`.
- [x] Validate with the engine's own `--dry-run` before touching the network stack.
- [x] nftables interception in a dedicated `inet blkbstr` table, so teardown is one atomic delete.
- [x] Locate the engine binary; refuse clearly, by name, when it is missing.
- [x] Supervise the process; notice an engine that exits by itself and pull the rules with it.
- [x] Tear rules down on stop, on `Drop`, and on startup after a kill that skipped both.
- [x] Restart a crashed engine, with backoff, and give up rather than thrash the firewall.
- [x] Start on boot with the last non-ephemeral config.
- [x] Uptime, pid and engine version in the GUI.
- [x] Log viewer: tail the engine and daemon logs in the GUI, with filter and one-click export.
- [ ] AppArmor and SELinux profiles, so hardened distros fail loudly instead of silently.

**Done when:** a Linux user installs, picks a config, and blocked sites load; rebooting keeps it
working; stopping leaves `nft list ruleset` exactly as it was before starting.

### M2 — Getting to a working config without knowing anything

The onboarding half. This is where the project earns its existence.

- Reachability check against a list of commonly blocked sites, with a per-site verdict
  (DNS poisoned / SNI reset / TLS blocked / fine).
- Auto-configuration: walk a ranked list of candidate strategies, apply each in ephemeral mode,
  re-test, keep the first that works. Save the winner under a user-chosen name.
- First-run wizard chaining detection → install → check → auto-configure, re-runnable later from
  the app.
- Import an existing zapret installation: detect it, back the current config up somewhere the
  user is told about, import it as a preset named `Legacy`.
- Ephemeral "try it" mode surfaced in the UI, with an automatic revert if the user does not
  confirm within a timeout.

**Done when:** someone who has never heard of DPI gets a working, saved, named config by clicking
through a wizard.

### M3 — Windows

- `winws2` backend for the same `Config` type. Interception is built into the process via
  WinDivert, so there are no firewall rules to install or remove — a smaller job than Linux.
- Windows Service installed by one UAC-elevated step; GUI talks to it over a named pipe with an
  ACL restricted to the installing user.
- Signed installer and binaries. Documented process for submitting to AV vendors, because
  anti-censorship tooling gets false-positived as a matter of routine.
- winget and Chocolatey packages.

**Done when:** the same config file works on Windows and Linux.

### M4 — Living with it

- Auto-update via Tauri's updater plugin against GitHub Releases, with the *engine* updated
  separately from the *app* — they have different risk profiles and different release cadences.
- System tray / menu bar toggle with a status indicator.
- Config sync to a user-supplied GitHub repo, private by default, with an explicit warning that a
  config reveals which sites someone visits and which strategies work in their country.
- Localisation. Not an afterthought: the userbase is disproportionately non-English-speaking.
- Accessibility pass — keyboard navigation, screen reader labels, contrast.

### M5 — FreeBSD and OpenBSD

The cheapest platform to add, because upstream already ships `dvtws2` for it.

- `dvtws2` backend, driven by the same `Config`; interception via ipfw (FreeBSD) or pf (OpenBSD).
- rc.d service script in place of the systemd unit.

There is no macOS milestone. Upstream does not support macOS and does not expect to — Apple
removed `ipdivert` from the kernel and there is no replacement — so it would mean writing a Network
Extension *and* a packet manipulator that does not exist upstream. That is a different project.

### Ongoing

Distribution packaging (Flatpak, AUR, deb repo, ports), contribution docs, and keeping the Lua
function registry in step as upstream zapret2 gains and loses functions.

## Standing constraints

These outrank milestone order.

1. **The GUI never runs as root.** If a feature seems to need it, the feature is wrong.
2. **Stopping restores the machine.** Firewall state on stop equals firewall state before start,
   including after a crash.
3. **A config that cannot fully apply still applies.** Unrecognised actions warn and are passed
   through to the engine; they never block the whole config from loading.
4. **No telemetry.** Not off-by-default: absent. Diagnostics are files the user chooses to attach.
5. **Nothing leaves the machine unasked.** Sync is opt-in, to the user's own repo, with the
   privacy implication spelled out first.

## Risks worth naming

| Risk | Why it matters | Response |
| --- | --- | --- |
| Upstream zapret2 API changes | Rendered actions silently change meaning | Configs record `NFQWS2_COMPAT_VER`; a mismatch warns. Upstream bumps it on every break |
| The Lua function registry goes stale | New upstream functions warn spuriously | Unknown functions are passed through, not rejected, so staleness costs a warning and nothing else |
| Lua runtime missing | The engine will not start, with an error that does not say why | Onboarding checks for LuaJIT 2.1+ / Lua 5.3+ before anything else |
| AV false positives on Windows | Users are told the anti-censorship tool is malware | Sign everything; start the vendor submission process before the M3 release, not after |
| Auto-config leaves a bad rule behind | The user's network breaks and they cannot fix it | Ephemeral mode with a confirm-or-revert timeout; teardown is tested, not assumed |
| Config sync leaks a threat profile | A config identifies both the user's country and their reading | Private repos, explicit warning, client-side encryption before it ships |
