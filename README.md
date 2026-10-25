# <img src="public/icon.svg" width="30" alt=""> Blockbuster

[![CI](https://github.com/dodoflix/blkbstr/actions/workflows/ci.yml/badge.svg)](https://github.com/dodoflix/blkbstr/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/dodoflix/blkbstr)](https://github.com/dodoflix/blkbstr/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A desktop GUI for [zapret2](https://github.com/bol-van/zapret2) - install it, configure it, run it
as a service, and see what it is doing, without opening a terminal.

Linux, Windows and the BSDs. Early: on Linux the engine runs, with nftables interception and
config management; onboarding and the Windows service are not built yet. See
[docs/plan.md](docs/plan.md).

## How it is put together

An unprivileged Tauri app talks over a local socket to a small privileged service that owns the
engine process and the firewall rules. Configs are portable JSON files owned by the user; the
service renders one into an nfqws2 parameter file, has the engine check it with `--dry-run`,
installs the interception rules and supervises the process.

- [docs/plan.md](docs/plan.md) - what we are building and in what order
- [docs/architecture.md](docs/architecture.md) - how the pieces fit
- [docs/development.md](docs/development.md) - running it, the daemon, troubleshooting
- [docs/](docs/) - config format, IPC protocol, platform notes, decisions
- [Contributing](CONTRIBUTING.md), [security policy](SECURITY.md)

## Layout

```
src/                    React frontend
src-tauri/              GUI backend: Tauri commands, daemon client, config files
crates/blkbstr-core/    Config model, Lua function registry, parameter-file renderer, protocol
crates/blkbstr-daemon/  blkbstrd - privileged helper: engine supervisor + nftables
packaging/linux/        systemd unit, polkit action, install scripts
docs/                   Plan, architecture, decisions
```

## Start

Needs Rust, Node and the [Tauri system dependencies](https://tauri.app/start/prerequisites/).

```sh
npm install && npm run tauri dev
```

Running the engine also needs zapret2 installed, `nftables` and a privileged service; see
[docs/development.md](docs/development.md).

## Privacy

No telemetry, of any kind. Nothing leaves the machine unless you turn on config sync, which
targets a repository you supply.

## Licence

MIT, matching upstream zapret2. See [LICENSE](LICENSE).
