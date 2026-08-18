# Changelog

Notable changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

A release cuts the `Unreleased` heading down to a version and date; the release workflow refuses a
tag with no section here. zapret2 is versioned separately, so a version below says nothing about
which engine you are running.

## [Unreleased]

### Added

- Runs the engine on Linux: renders a config to an nfqws2 parameter file, checks it with the
  engine's own `--dry-run`, installs nftables interception in a dedicated `inet blkbstr` table and
  supervises the process.
- Restarts a crashed engine with backoff and gives up after a crash loop rather than thrashing the
  firewall. Rules come down with the engine, including on `Drop` and on the next startup after a
  kill that skipped both.
- Brings back the last non-ephemeral config on boot.
- Portable JSON configs with named strategies and ordered Lua actions, validated before anything
  reaches the network stack. Unknown Lua functions warn and are passed through.
- Status panel with uptime, pid and engine version; log viewer with filter, follow and one-click
  diagnostic export.
- Service control from the GUI — enable at boot, start and stop now — through polkit.
- systemd unit, polkit action and install/uninstall scripts, plus optional AppArmor and SELinux
  profiles.

### Fixed

- The installed service could not start at all: the unit's capability bounding set dropped
  `CAP_CHOWN`, so putting the socket in the `blkbstr` group failed with `EPERM` and the daemon
  exited.
- `MemoryDenyWriteExecute=yes` in the unit killed LuaJIT, and zapret2's attacks are Lua — the
  engine died with `runtime code generation failed, restricted kernel?` before seeing a packet.
- `ProtectHome=yes` hid `/home`, so a config pointing `hostlist=` at a file the user picked was
  rejected as missing.
- The window did not open on the NVIDIA proprietary driver under Wayland
  (`Error 71 (Protocol error)`): WebKitGTK's DMA-BUF renderer is now disabled on Linux.

### Security

- Config values containing a newline are rejected. nfqws2 reads options from a file one per line,
  so a newline in an imported or synced config would inject an arbitrary option into a process that
  manipulates the firewall.
- The socket's mode and group are the authorisation model: `0660`, group `blkbstr`. The GUI never
  runs elevated.
