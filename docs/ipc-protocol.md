# IPC protocol

Newline-delimited JSON over a local socket. Defined in `crates/blkbstr-core/src/protocol.rs`;
that file is the authority, this page is the explanation.

## Transport

| OS | Endpoint |
| --- | --- |
| Linux, BSD | Unix socket `/run/blkbstr/blkbstrd.sock`, mode `0660`, group `blkbstr` |
| Windows | Named pipe `blkbstrd.sock` |

One request and one response per connection; the GUI opens a new connection each time. Both sides
flush after writing — a half-written line with both ends waiting is a deadlock, not an error.

Access to the socket is the entire authorisation model, so its permissions are load-bearing: the
daemon sets `0660` immediately after binding and hands it to the group given by
`--socket-group-gid`. Without that flag it stays root-only and logs a warning, which is a
GUI that cannot connect rather than a socket anyone can drive.

> Windows named-pipe ACLs are not implemented yet. The pipe must be restricted to the installing
> user's group before any Windows release — see `bind()` in `crates/blkbstr-daemon/src/main.rs`.

## Versioning

Every `Ping` carries `PROTOCOL_VERSION`. A mismatch is answered with `protocol_mismatch` rather
than a best-effort guess, so the GUI can say "your service is older than your app" instead of
failing in an interesting way three requests later.

## Requests

| Request | Fields | Effect |
| --- | --- | --- |
| `ping` | `protocol` | Liveness plus version handshake. Also how the GUI learns the service is installed at all |
| `status` | — | Current engine state |
| `start` | `config`, `ephemeral` | Apply a config and start the engine |
| `stop` | — | Stop the engine and restore the firewall |

`ephemeral: true` runs the config without recording it as active, so a failed experiment is undone
by a restart. This is the transport half of the "try it" mode in the UI.

`status` never fails. A daemon with no engine — because zapret2 is not installed — still answers,
with the reason in `last_error`, so the GUI can explain the situation instead of rendering a failed
request.

```json
{"op":"ping","protocol":1}
{"op":"start","config":{"schema":1,"name":"home-isp","strategies":[]},"ephemeral":true}
```

## Responses

```json
{"result":"pong","daemon_version":"0.1.0","protocol":1}
{"result":"status","running":true,"active_config":"home-isp","ephemeral":false,"pid":4211}
{"result":"status","running":false,"ephemeral":false,"last_error":"engine exited: signal: 9"}
{"result":"ok"}
{"result":"error","code":"engine_failed","message":"nfqws2 exited with status 1"}
```

| Code | Meaning |
| --- | --- |
| `protocol_mismatch` | Daemon and GUI disagree on the protocol version |
| `bad_request` | Malformed request or invalid config |
| `engine_failed` | The engine itself failed — binary missing, config rejected by `--dry-run`, rules refused |

## What is not here

**Log streaming.** The daemon writes files and the GUI tails them. Putting a log subscription on
this socket would add framing, backpressure and per-connection subscription state to the
privileged process in exchange for reading a file that is already readable.

**Config storage.** Configs are user files the GUI owns. The daemon sees a config only as an
argument to `start`, and never reads or writes the config directory.

Both omissions are the same principle: the privileged surface stays at "start, stop, report".
