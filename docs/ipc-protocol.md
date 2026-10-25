# IPC protocol

Newline-delimited JSON over a local socket. Defined in `crates/blkbstr-core/src/protocol.rs`;
that file is the authority, this page is the explanation.

## Transport

| OS | Endpoint |
| --- | --- |
| Linux, BSD | Unix socket `/run/blkbstr/blkbstrd.sock`, mode `0660`, group `blkbstr` |
| Windows | Named pipe `blkbstrd.sock` |

One request and one response per connection; the GUI opens a new connection each time. Both sides
flush after writing - a half-written line with both ends waiting is a deadlock, not an error.

Access to the socket is the entire authorisation model, so its permissions are load-bearing: the
daemon sets `0660` immediately after binding and hands it to the group given by
`--socket-group-gid`. Without that flag it stays root-only and logs a warning, which is a
GUI that cannot connect rather than a socket anyone can drive.

On Windows the pipe's access list is SYSTEM and the one user who ran `blkbstrd install-service`,
passed to the service as `--allow-sid`. Not Administrators: the pipe is the authority to run the
engine, and it belongs to a person. Remote clients are refused, and a service started without
`--allow-sid` stops instead of serving the pipe's default access. Installing again is how access
moves to a different account.

## Versioning

Every `Ping` carries `PROTOCOL_VERSION`. A mismatch is answered with `protocol_mismatch` rather
than a best-effort guess, so the GUI can say "your service is older than your app" instead of
failing in an interesting way three requests later.

## Requests

| Request | Fields | Effect |
| --- | --- | --- |
| `ping` | `protocol` | Liveness plus version handshake. Also how the GUI learns the service is installed at all |
| `status` | - | Current engine state |
| `start` | `config`, `ephemeral` | Apply a config and start the engine |
| `confirm` | - | Keep a trial run: cancels its revert and records it as active |
| `stop` | - | Stop the engine and restore the firewall |
| `hosts` | `list` | A host list (`auto` or `manual`) and, for `auto`, the state of its re-check |
| `hosts_add` | `list`, `hosts` | Add hosts; anything that is not a plain hostname is ignored |
| `hosts_remove` | `list`, `hosts` | Remove hosts. The engine learns an `auto` host again if it still fails |
| `hosts_check` | - | Start a re-check of the `auto` list now; returns at once |

`ephemeral: true` runs the config without recording it as active, so a failed experiment is undone
by a restart. It also arms a deadline: unless a `confirm` arrives within two minutes, the daemon
stops the engine and removes the rules by itself, and `status` reports why.

That deadline lives in the daemon rather than the GUI on purpose. The case it exists for is the
one where the GUI cannot help - a strategy that takes the network down with it, or a client that
died with the rules still up. A timer in the process that owns the firewall is the only one that
still runs then. `BLKBSTR_REVERT_SECONDS` shortens it for testing.

`status` never fails. A daemon with no engine - because zapret2 is not installed - still answers,
with the reason in `last_error`, so the GUI can explain the situation instead of rendering a failed
request.

```json
{"op":"ping","protocol":3}
{"op":"start","config":{"schema":1,"name":"home-isp","strategies":[]},"ephemeral":true}
```

## Responses

```json
{"result":"pong","daemon_version":"0.1.0","protocol":1}
{"result":"status","running":true,"active_config":"home-isp","ephemeral":true,"revert_in_seconds":97,"pid":4211}
{"result":"status","running":false,"ephemeral":false,"last_error":"engine exited: signal: 9"}
{"result":"hosts","list":"auto","enabled":true,"hosts":["discord.com"],"checking":false,"last_dropped":[]}
{"result":"ok"}
{"result":"error","code":"engine_failed","message":"nfqws2 exited with status 1"}
```

| Code | Meaning |
| --- | --- |
| `protocol_mismatch` | Daemon and GUI disagree on the protocol version |
| `bad_request` | Malformed request or invalid config |
| `engine_failed` | The engine itself failed - binary missing, config rejected by `--intercept=0`, rules refused |

## What is not here

**Log streaming.** The daemon writes files and the GUI tails them. Putting a log subscription on
this socket would add framing, backpressure and per-connection subscription state to the
privileged process in exchange for reading a file that is already readable.

**Config storage.** Configs are user files the GUI owns. The daemon sees a config only as an
argument to `start`, and never reads or writes the config directory.

**Engine installation.** `blkbstrd install-engine` is a command-line one-shot run through `pkexec`,
not a request: a socket operation is open to the whole `blkbstr` group, and writing an executable
that later runs as root should ask for a password. See [architecture.md](architecture.md#installing-the-engine).

All three omissions are the same principle: the privileged surface stays at "start, stop, report".
