# Config format

A config is a JSON file the user owns, names, saves, switches between and optionally syncs. It is
not a command line: the daemon renders it into an nfqws2 parameter file, which the engine reads
with its `@<file>` option.

Files live in `configs/` under the platform config directory, one file per config, named after
the config:

| OS | Directory |
| --- | --- |
| Linux, BSD | `~/.config/blkbstr/configs/` |
| Windows | `%APPDATA%\blkbstr\blkbstr\config\configs\` |

## Shape

It mirrors zapret2's own vocabulary. A **strategy** is one nfqws2 *profile*; its **actions** are
the ordered `--lua-desync` calls inside that profile.

```json
{
  "schema": 1,
  "compat": 3,
  "name": "home-isp",
  "notes": "Works on fibre, not on the mobile hotspot.",
  "strategies": [
    {
      "name": "https",
      "enabled": true,
      "filter": { "tcp": "443", "l7": ["tls"], "hostlist": "/etc/blkbstr/blocked.txt" },
      "actions": [
        {
          "function": "fake",
          "payload": ["tls_client_hello"],
          "args": { "blob": "fake_default_tls", "badsum": "", "strategy": "1" }
        },
        {
          "function": "multidisorder",
          "payload": ["tls_client_hello"],
          "args": { "strategy": "2" }
        }
      ]
    }
  ]
}
```

Which renders to:

```
--qnum=200
--new=https
--filter-tcp=443
--filter-l7=tls
--hostlist=/etc/blkbstr/blocked.txt
--payload=tls_client_hello
--lua-desync=fake:badsum:blob=fake_default_tls:strategy=1
--payload=tls_client_hello
--lua-desync=multidisorder:strategy=2
```

`preview_config` returns exactly this, so the UI can show it rather than asking the user to take
the config on faith.

### Actions are ordered

`actions` is a list, not a map, because nfqws2 runs them in sequence and the orchestrators
(`circular`, `repeater`, `condition`) are built on that order. Two actions can name the same
function with different arguments, which a map could not express either.

### Arguments

`args` renders as `:key=value`. An **empty value is a bare flag**: `{"badsum": ""}` becomes
`:badsum`, matching how the manual documents `fake:blob=...:badsum`.

## Versioning

Two independent numbers:

- **`schema`** — this file format. Higher than the running build is rejected rather than
  half-read.
- **`compat`** — the `NFQWS2_COMPAT_VER` the actions were written against. Upstream bumps it on
  every API break (v2 replaced the `stun_binding_req` payload, v3 restructured `desync.track`). A
  mismatch *warns*: the actions may still work, but their arguments may have changed meaning.

They are separate because our file format and upstream's engine API move independently.

## Validation

Configs arrive by hand-editing, by import, and eventually by sync from a repository. They are
untrusted input, and two checks exist because of what they feed:

**Names become filenames.** Empty names, names containing separators, and dot-prefixed names are
refused. A config called `../../.bashrc` must not be a way to write to `../../.bashrc`.

**Values become lines in a parameter file.** nfqws2 reads options one per line, so a newline in a
hostlist path would inject an arbitrary option into a process that rewrites the firewall. Newlines,
carriage returns and NULs are refused in every value. Inside `--lua-desync`, `:` separates
arguments, so it is refused in argument values too. Function names, strategy names and argument
keys must be bare identifiers.

Both sides validate. The GUI validates before saving and after loading; the daemon validates again
before acting, because a privileged process does not trust its caller.

## Linting

`lint()` reports what will not do what it appears to. It never fails a config.

- An action naming a Lua function this build does not know — usually a typo. It is **passed
  through**, not dropped, because it may be a function from a newer upstream release.
- A `compat` mismatch.
- An enabled strategy with no actions: it matches traffic and does nothing to it.

The authority on whether a config actually loads is the engine's own `--dry-run`, which the daemon
runs before touching the network stack. It checks options and file existence; it does **not**
validate Lua, so a config can pass and still fail at runtime.

## Platform portability

**A config is portable as written.** The attacks are Lua, so they do not vary by platform; only the
interception layer does — `--qnum` on Linux, `--wf-*` on Windows, `--port` on BSD — and none of
that is user config, because the daemon supplies it. See
[adr/0004-portable-config-that-warns.md](adr/0004-portable-config-that-warns.md).

## Adding a function

Add a `FunctionSpec` to `FUNCTIONS` in `crates/blkbstr-core/src/registry.rs`. Nothing is rendered
from that table by name, so an out-of-date list costs a spurious warning rather than a broken
config.

Bumping `SCHEMA_VERSION` is only for changes that make old files unreadable.
