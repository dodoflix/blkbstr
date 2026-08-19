# <img src="public/icon.svg" width="30" alt=""> Blockbuster

A desktop GUI for [zapret2](https://github.com/bol-van/zapret2) — install it, configure it, run it
as a service, and see what it is doing, without opening a terminal.

Linux, Windows and the BSDs. Early: on Linux the engine runs, with nftables interception and
config management; onboarding and the Windows service are not built yet. See
[docs/plan.md](docs/plan.md).

## How it is put together

An unprivileged Tauri app talks over a local socket to a small privileged service that owns the
engine process and the firewall rules. Configs are portable JSON files owned by the user; the
service renders one into an nfqws2 parameter file, has the engine check it with `--dry-run`,
installs the interception rules and supervises the process.

- [docs/plan.md](docs/plan.md) — what we are building and in what order
- [docs/architecture.md](docs/architecture.md) — how the pieces fit
- [docs/](docs/) — config format, IPC protocol, platform notes, decisions

## Layout

```
src/                    React frontend
src-tauri/              GUI backend: Tauri commands, daemon client, config files
crates/blkbstr-core/    Config model, Lua function registry, parameter-file renderer, protocol
crates/blkbstr-daemon/  blkbstrd — privileged helper: engine supervisor + nftables
packaging/linux/        systemd unit, polkit action, install scripts
docs/                   Plan, architecture, decisions
```

## Building

Needs Rust, Node and the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for
your platform. Running the engine additionally needs zapret2 installed (`nfqws2` on `PATH` or in
`/opt/zapret2`), `nftables`, and LuaJIT 2.1+ or Lua 5.3+.

```sh
npm install
npm run tauri dev          # run the GUI
cargo test --workspace     # Rust tests
npm run build              # type-check and build the frontend
```

The app icon comes from `public/icon.svg`, which is also the favicon. `tauri icon` cannot read SVG,
so regenerating the platform icon set means rasterising first:

```sh
rsvg-convert -w 1024 -h 1024 public/icon.svg -o /tmp/icon.png
npm run tauri -- icon /tmp/icon.png
rm -rf src-tauri/icons/android src-tauri/icons/ios   # desktop only
```

## Running the daemon

Nothing can start or stop the engine without the privileged service.

```sh
cargo build --release -p blkbstr-daemon
sudo packaging/linux/install.sh
```

The installer creates a `blkbstr` group, adds you to it, and installs a systemd unit. **Log out
and back in** — group membership only applies to new sessions, so until you do, the GUI will
report the service as unreachable.

Remove it with `sudo packaging/linux/uninstall.sh`.

AppArmor and SELinux profiles ship alongside it, loaded by hand rather than by the installer — see [packaging/linux/README.md](packaging/linux/README.md).

To run the daemon unprivileged while developing, point both halves somewhere writable:

```sh
export BLKBSTR_SOCKET=/tmp/blkbstr/d.sock
export BLKBSTR_STATE_DIR=/tmp/blkbstr/state
export BLKBSTR_RUNTIME_DIR=/tmp/blkbstr/run
export BLKBSTR_LOG_DIR=/tmp/blkbstr/logs
cargo run -p blkbstr-daemon
npm run tauri dev
```

Applying the rules still needs root, so an unprivileged daemon gets as far as `nft` and stops
there. To go further, run the privileged daemon from `target/debug` instead:

```sh
npm run dev:daemon              # builds, stops the service, runs it in the foreground
npm run watch:daemon            # the same, rebuilt and restarted whenever a .rs file changes
```

It takes over the installed service's socket and hands it to your own primary group, so the GUI
reaches it without the `blkbstr` group and without logging out. `sudo systemctl start blkbstrd`
puts the installed one back.

Windows and the BSDs have no service installer yet; see
[docs/platform-notes.md](docs/platform-notes.md).

## Troubleshooting

**The window does not open, and the log says
`Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.`** — WebKitGTK's DMA-BUF
renderer fails on the NVIDIA proprietary driver under Wayland. The app already sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` for itself on Linux; if you have exported that variable as `0`
in your shell, that override wins and this is why.

**The GUI says the service is not reachable right after installing.** Group membership only applies
to new sessions. Log out and back in, or check with `id -nG | grep blkbstr`.

## Privacy

No telemetry, of any kind. Nothing leaves the machine unless you turn on config sync, which
targets a repository you supply.

## Licence

MIT, matching upstream zapret2. See [LICENSE](LICENSE).
