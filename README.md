# Blockbuster

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

To run the daemon unprivileged while developing, point both halves somewhere writable:

```sh
BLKBSTR_SOCKET=/tmp/blkbstrd.sock cargo run -p blkbstr-daemon
BLKBSTR_SOCKET=/tmp/blkbstrd.sock npm run tauri dev
```

Windows and the BSDs have no service installer yet; see
[docs/platform-notes.md](docs/platform-notes.md).

## Privacy

No telemetry, of any kind. Nothing leaves the machine unless you turn on config sync, which
targets a repository you supply.

## Licence

MIT, matching upstream zapret2. See [LICENSE](LICENSE).
