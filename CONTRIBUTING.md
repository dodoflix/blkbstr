# Contributing

## Before you start

Read [docs/plan.md](docs/plan.md) for what is in scope and [docs/backlog.md](docs/backlog.md) for
what is already planned. For anything larger than a fix, open an issue first — the milestones are
deliberately ordered, and a good PR against M4 while M1 is unfinished is still a PR that sits.

## Setup

```sh
npm install
cargo test --workspace
npm run tauri dev
```

## Checks

CI runs these on Linux and Windows, the two platforms with hosted runners that we target. Run them
before pushing:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run build
```

## What the code should look like

**Non-trivial logic leaves a test behind.** Not a suite — the smallest thing that fails if the
logic breaks. The existing tests in `blkbstr-core` are the model.

**Comments explain what the code cannot.** A confirmed constraint, a non-obvious reason an
"obvious" simplification is wrong, something a reader would be surprised by. Not a restatement of
the line below it.

**The privileged half stays small.** New dependencies in `blkbstr-daemon` need a reason;
everything in that crate runs as root. If a feature can live in the GUI, it lives in the GUI.

**The function registry is honest.** It lists what upstream zapret2 documents, nothing more.
Nothing is rendered from it by name, so a missing entry costs a warning — but an invented entry
offers the user a strategy that cannot work.

## Things that need care

- **Firewall teardown.** Rules must come down on stop, on `Drop`, and on the next startup after a
  kill that skipped both. A change to the engine backend that breaks any of the three strands
  people with a network that half works.
- **Order of operations.** Validate, `--dry-run`, *then* rules, *then* the engine. On the way down,
  rules first. Rules pointing at a queue nothing reads is the state that breaks a machine.
- **Socket permissions.** Anyone who can write the socket can rewrite the firewall.
- **Config names.** They become filenames. `validate()` exists for a reason and both sides call it.
- **Config values.** They become lines in the engine parameter file; a newline injects an arbitrary
  option into a root process. Configs arrive by import and by sync — treat them as untrusted.
- **No telemetry.** Not off-by-default, not opt-in-later. Absent.

## Commits and PRs

Subject line says what changed. Body only if the reason is not obvious from the diff. Keep PRs to
one thing; a refactor and a fix in the same PR is two reviews pretending to be one.
