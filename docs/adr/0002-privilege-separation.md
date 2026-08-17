# 2. A separate privileged daemon, with the socket as the permission

Status: accepted

## Context

zapret2 manipulates the packet path and needs elevation. The GUI does not. Something has to decide
where the boundary is and how the unprivileged half is authorised across it.

## Decision

A small privileged service (`blkbstrd`) owns the engine and the firewall rules. The GUI runs as an
ordinary user process and asks it for four operations over a local socket. Access to the socket is
the authorisation: on Linux the socket is `0660` owned by the `blkbstr` group.

Elevated: start, stop, firewall rules, installing or removing the service.
Unelevated: status, logs, and everything to do with configs.

## Why

Running the whole app elevated would put a browser engine, a JavaScript runtime and the entire npm
dependency tree in a root process. The privileged binary instead depends on `serde`, `interprocess`
and `tracing`, and can be read end to end.

Group membership as the permission avoids the two bad alternatives: prompting for a password on
every toggle, which trains users to approve prompts reflexively, and a permanent passwordless
`sudo` rule, which is a broader grant with no obvious way to revoke it. Removing someone from a
group is a thing an administrator already knows how to do.

The polkit action covers only enabling the service at boot — a genuinely administrative change —
rather than the day-to-day start/stop.

## Costs

Group membership takes effect at next login, which is a confusing first-run experience and the
installer says so explicitly.

An IPC layer is code that would not otherwise exist: a protocol, a client, a server, and version
negotiation between them.

The socket permissions are load-bearing. A daemon started without `--socket-group-gid` leaves a
root-only socket and a GUI that cannot connect; it logs a warning rather than falling back to
something permissive, because the failure mode of getting this wrong in the other direction is
that any local process can rewrite the firewall.

## Alternatives

**One elevated process** — no IPC, but a root-owned browser engine.

**`sudo` per action** — no daemon, but prompt fatigue and no persistent service to restart the
engine after a crash.

**setuid helper binary** — smaller than a service, but no supervision, no state, and setuid
binaries are their own well-documented category of mistake.
