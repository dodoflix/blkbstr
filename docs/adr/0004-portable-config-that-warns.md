# 4. A portable config that warns instead of refusing

Status: accepted

## Context

The same config should be usable on a user's laptop and their desktop, and shareable with someone
in the same country on a different OS. But a config can name a Lua function, a payload type or an
argument that the running build does not recognise — because it was written by a newer build, or
because it is a typo. Something has to happen then.

## Decision

One config format for every platform. Anything the build does not recognise produces a warning and
is **passed through to the engine**. It never prevents the config from loading.

## Why

Rejecting a config that contains anything unrecognised would make every older build useless against
every newer config, and make a portable format unportable in practice.

A build cannot tell whether a name it has not heard of is a typo or the future. Passing it through
lets the engine decide, and nfqws2's own `--dry-run` is a better authority on what the engine
accepts than any table of ours.

Portability itself is close to free here, because the attacks are Lua rather than compiled into
each platform's binary. What differs between `nfqws2`, `winws2` and `dvtws2` is interception —
`--qnum`, `--wf-*`, `--port` — and none of that is user config; the daemon supplies it. So the
check that remains is not "does this platform support this parameter" but "does this function
exist at all", which is a registry rather than a matrix.

The registry is deliberately not load-bearing: nothing is rendered from it by name, so letting it
fall behind upstream costs a spurious warning rather than a broken config.

## Costs

Warnings can be ignored, and a user whose config quietly does nothing may not understand why. The
UI has to show them at the point of use, not bury them in a log.

Passing unknown names through means typos are not caught by the type system. Warnings are the
mitigation, which is weaker than a compile error and is the price of forward compatibility.

The registry is hand-maintained and will drift from upstream. It is tested for internal
consistency, but nothing checks it against the shipped Lua libraries — keeping it in step is an
explicit ongoing task in the backlog.

## Alternatives

**Per-platform config files** — no portability, and no sync worth having.

**Strict schema, reject unknown names** — typos caught immediately, at the cost of old builds
refusing new files.

**Store the rendered parameter file instead of a model** — trivial, and unreadable to the UI: no
editor, no linting, no preview, and no way to move a config to a platform whose interception
differs.
