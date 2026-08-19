# Changelog

Notable changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

A release cuts the `Unreleased` heading down to a version and date; the release workflow refuses a
tag with no section here. zapret2 is versioned separately, so a version below says nothing about
which engine you are running.

## [Unreleased]

### Added

- Runs the engine on Linux: renders a config to an nfqws2 parameter file, checks it by loading it
  with `--intercept=0`, installs nftables interception in a dedicated `inet blkbstr` table and
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
- Auto-configuration over 113 candidate strategies, following the sweep zapret2's own `blockcheck2`
  runs: nine split positions across two split functions, eight TCP foolings against the faked-split
  family, autottl deltas and fixed TTLs, `wssize`, `syndata`, `tcpseg` and `oob`. Every one of them
  is tried, and the walk can be stopped partway with an answer.
- Every strategy that worked is then measured against the sites that already worked — median
  handshake time over three rounds, and anything that stopped working — and the results are offered
  as a choice rather than a single recommendation.
- Saved configs can be run, previewed and deleted from the Configs tab.
- `scripts/dev-daemon.sh`, which runs the daemon from `target/debug` with the socket handed to the
  caller's own primary group, so development needs neither the `blkbstr` group nor a logout.
  `--watch` rebuilds and restarts it on change.
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
- Every strategy was a no-op. nfqws2 has profile 1 open before it reads any option, so `--new` for
  the first strategy left that one empty — and an empty profile has no filters, matches every packet
  first and passes it through untouched.
- No strategy could load: zapret2's desync functions live in its Lua library, which was never passed
  with `--lua-init`, so every action failed with `desync function 'multisplit' does not exist`.
- The config check was starting the engine for real. nfqws2 documents `@<file>` as "must be the only
  argument. other options are ignored", and the `--dry-run` after it was silently dropped.
- Splits ran at the default position of two bytes in. Candidates passed `strategy=N`, which is only
  read by the `circular` orchestrator, so the argument that decides where a split lands was never
  set.
- Killing the daemon left the engine running with NFQUEUE still bound, after which every start died
  with `nfq_create_queue(): Operation not permitted`. The daemon now stops the engine on
  `SIGTERM`/`SIGINT`/`SIGHUP` and kills one left behind by a previous daemon at startup.
- `Permission denied (os error 13)` on the socket now says which group is missing and that a session
  has to be logged out and back in for `usermod -aG` to reach it.
- Uninstalling deleted the `blkbstr` group, so reinstalling could allocate a different GID and cost
  the user another logout.
- The window did not open on the NVIDIA proprietary driver under Wayland
  (`Error 71 (Protocol error)`): WebKitGTK's DMA-BUF renderer is now disabled on Linux.

### Security

- Config values containing a newline are rejected. nfqws2 reads options from a file one per line,
  so a newline in an imported or synced config would inject an arbitrary option into a process that
  manipulates the firewall.
- The socket's mode and group are the authorisation model: `0660`, group `blkbstr`. The GUI never
  runs elevated.
