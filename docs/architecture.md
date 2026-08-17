# Architecture

## The split

```
┌──────────────────────────────┐        ┌──────────────────────────────┐
│  blkbstr  (unprivileged)     │        │  blkbstrd  (root / SYSTEM)   │
│                              │        │                              │
│  React + Radix Themes        │        │  socket server               │
│      │ invoke()              │        │      │                       │
│  src-tauri                   │        │  engine.rs   (supervisor)    │
│   ├ daemon.rs  ──────────────┼── JSON ┼─▶ nfqws2 / winws2 / dvtws2   │
│   ├ configs.rs (user files)  │  lines │      │                       │
│   └ commands                 │        │  firewall.rs (nft table)     │
└──────────────────────────────┘        └──────────────────────────────┘
        ~/.config/blkbstr                       /var/log/blkbstr
        configs, backups                        rolling logs  ──┐
              ▲                                                 │
              └──────────── read-only tail ─────────────────────┘
```

Both halves depend on `blkbstr-core`, which holds the config model, the Lua function registry and
the protocol types. Anything both sides must agree on lives there; nothing else does.

## Why two processes

zapret2 manipulates the packet path — nfqueue plus nftables on Linux, WinDivert on Windows, ipfw
or pf on the BSDs. That needs elevation. A GUI does not.

Running the whole app elevated would mean a browser engine, a JavaScript runtime and every
npm dependency executing as root, which is a large attack surface for the sake of skipping an IPC
layer. So the privileged work is a small, dependency-light service, and the GUI is an ordinary
user program that can only ask it for four things.

The boundary is also the permission model. Access to the socket *is* the authorisation: on Linux
the socket is `0660` owned by the `blkbstr` group, and being in that group is what lets you drive
the engine. No repeated `sudo`, no password prompt per toggle, and a clear thing to revoke.

**What needs elevation:** starting and stopping the engine, changing firewall rules, installing or
removing the service. **What does not:** reading status, reading logs, and everything to do with
configs — creating, editing, validating, linting, importing, syncing. Config files live in the
user's own directory and are only handed to the daemon at the moment of starting.

## Crates

| Path | Crate | Contents |
| --- | --- | --- |
| `crates/blkbstr-core` | `blkbstr-core` | `Config`, the Lua function registry and `lint`, the parameter-file renderer, the wire protocol, shared paths |
| `crates/blkbstr-daemon` | `blkbstr-daemon` (bin `blkbstrd`) | Socket server, engine supervision, nftables interception |
| `src-tauri` | `blkbstr` | Tauri commands, daemon client, config file CRUD |
| `src` | — | React frontend |

## Request flow

Starting the engine, end to end:

1. The user picks a config. The GUI calls `lint_config` and `preview_config`, both pure and local,
   and can show the warnings and the exact parameter file before anything happens.
2. The GUI calls `engine_start`, which opens a fresh socket connection and sends one
   `Request::Start`.
3. The daemon validates the config, logs the same warnings from its own side, writes the parameter
   file, and runs the engine with `--dry-run` to have the engine itself check it.
4. Only then does it install the nftables rules and spawn the engine. Rules go last because rules
   pointing at a queue nothing reads is the state that breaks the user's network.
5. The daemon replies `Ok`, or `Error` with a code the UI can act on.
6. The GUI polls `engine_status` for pid, uptime, engine version and the last error.

Teardown reverses it: rules first, then the process. The `Firewall` value also removes the table on
`Drop`, and the daemon clears a stale table at startup, so the three ways of dying — clean stop,
panic, SIGKILL — all end with the machine as it was.

## Supervision

A monitor thread checks on the engine every two seconds. When it finds one that exited by itself it
removes the rules immediately — rules must not outlive the process reading the queue, not even for
the seconds until a restart — and then asks `supervisor.rs` what to do.

The policy is a sliding window: five restarts within sixty seconds, with backoff of 1s, 2s, 4s, 8s,
15s, and then it gives up and leaves the engine down with the reason in `last_error`. Giving up is
the point of having a policy at all. Each restart rewrites the nftables ruleset, so an engine dying
instantly on a bad config would otherwise flap the user's network indefinitely. A deliberate start
or stop resets the window, because the user is usually fixing exactly what crashed.

An ephemeral run is never restarted: if an experiment dies, it has answered the question.

## Surviving a reboot

After a successful non-ephemeral start, the daemon writes the config to `state_dir()`
(`/var/lib/blkbstr`). At startup it reads it back, re-validates it, and starts it. `stop` deletes
it, because stopping is a decision to stay stopped.

It is written only *after* the engine is up, so a config that cannot run is never the one the
machine tries to bring up at boot. It is re-validated on the way back in for the same reason
everything else is: that file decides what a root process runs.

One connection per request. The daemon is a service that can restart underneath a running GUI, so
a persistent connection would buy reconnect logic and nothing else.

## Logging

The daemon writes rolling daily files to `/var/log/blkbstr` (`C:\ProgramData\blkbstr\logs` on
Windows) and to stderr, so the service manager captures them too. The GUI tails the files directly
rather than streaming over the socket — a log stream would mean framing, backpressure and
subscription state on the privileged side, in exchange for reading a file the user can already
read.

Filtering the daemon's own output is `BLKBSTR_LOG` (`tracing-subscriber` env-filter syntax),
defaulting to `info`.

The GUI's Logs tab lists the files, re-reads the last 256 KB of the selected one every two seconds
while "Follow" is on, and filters lines client-side. Export writes a single file — status plus
every log — into the user's config directory and reveals it. It is never uploaded: engine logs name
the hosts they saw, so the user reads it before deciding to send it anywhere.

## Frontend

React with Radix Themes. Component-level props and theme tokens carry the styling; there is no
Tailwind and no CSS framework to learn. See [adr/0003-radix-themes-over-shadcn.md](adr/0003-radix-themes-over-shadcn.md).

`src/api.ts` is the only file that calls `invoke`. It is a hand-written mirror of the Tauri
commands, so a signature change in Rust is a compile error in exactly one place.
