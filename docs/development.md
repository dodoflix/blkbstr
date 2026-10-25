# Development

Building and running the app and the daemon from a checkout.

## Layout

```
src/                    React frontend (src/ui is the design system)
src-tauri/              GUI backend: Tauri commands, daemon client, config files
crates/blkbstr-core/    Config model, Lua function registry, parameter-file renderer, protocol
crates/blkbstr-daemon/  blkbstrd, the privileged helper: engine supervisor and nftables
packaging/linux/        systemd unit, polkit actions, install scripts
docs/                   Plan, architecture, decisions
```

## Building

Needs Rust, Node and the [Tauri system dependencies](https://tauri.app/start/prerequisites/) for
your platform. Running the engine additionally needs `nftables` and LuaJIT 2.1+ or Lua 5.3+, plus
zapret2 itself: the setup wizard installs it, or put `nfqws2` on `PATH` or in `/opt/zapret2` yourself.

```sh
npm install
npm run tauri dev          # run the GUI
npm run dev                # the UI alone in a browser, answered by a mock backend
cargo test --workspace     # Rust tests
npm run build              # type-check and build the frontend
```

The app icon comes from `public/icon.svg`, which is also the favicon. Regenerate the platform icon
set with `npm run tauri -- icon public/icon.svg -o /tmp/icons` and copy the files that already exist in
`src-tauri/icons/` (desktop only, so leave out the `android` and `ios` folders).

## Running the daemon

Nothing can start or stop the engine without the privileged service.

```sh
cargo build --release -p blkbstr-daemon
sudo packaging/linux/install.sh
```

The installer creates a `blkbstr` group, adds you to it, and installs a systemd unit. **Log out
and back in** - group membership only applies to new sessions, so until you do, the GUI will
report the service as unreachable.

Remove it with `sudo packaging/linux/uninstall.sh`.

AppArmor and SELinux profiles ship alongside it, loaded by hand rather than by the installer - see [packaging/linux/README.md](packaging/linux/README.md).

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

The wizard's engine install runs `pkexec /usr/local/libexec/blkbstrd install-engine`, so it needs
`install.sh` to have run (the binary and the polkit action); `dev:daemon` alone does not provide them.

Windows and the BSDs have no service installer yet; see [platform-notes.md](platform-notes.md).

## Taskbar icon in `tauri dev`

On Wayland a window cannot carry its own icon: the compositor takes the app id (`blkbstr`) and looks
for a desktop entry of that name. Packaged builds have one; a dev run does not. Install a hidden one
that points at the repo with `scripts/dev-desktop.sh` (`--remove` to undo).

## The UI developer page

A dev build adds a Developer item to the sidebar: every component, the design tokens, the real
screens over fixed fake backend state, and the app in iframes of exact window sizes. It is compiled
out of release builds. See [design-system.md](design-system.md).

## Troubleshooting

**The window does not open, and the log says
`Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.`** - WebKitGTK's DMA-BUF
renderer fails on the NVIDIA proprietary driver under Wayland. The app already sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` for itself on Linux; if you have exported that variable as `0`
in your shell, that override wins and this is why.

**The GUI says the service is not reachable right after installing.** Group membership only applies
to new sessions. Log out and back in, or check with `id -nG | grep blkbstr`.
