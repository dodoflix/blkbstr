# Contributing

## Before you start

Read [docs/plan.md](docs/plan.md) for what is in scope and [docs/backlog.md](docs/backlog.md) for
what is already planned. For anything larger than a fix, open an issue first - the milestones are
deliberately ordered, and a good PR against M4 while M1 is unfinished is still a PR that sits.

## Set up

```sh
npm install        # also points git at .githooks
cargo test --workspace
npm run tauri dev  # the app, with a mock backend in a plain browser: npm run dev
```

Needs Rust, Node 22+ and the [Tauri system dependencies](https://tauri.app/start/prerequisites/).
[docs/development.md](docs/development.md) covers running the privileged daemon, the dev taskbar
icon and the UI developer page.

`npm install` sets `core.hooksPath` to `.githooks`: `rustfmt` and a type-check run on staged files
before every commit, and clippy, the Rust tests and a frontend build
run before a push when those parts changed. `--no-verify` skips them once; CI runs them regardless.

## Checks

Run these before pushing:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run build
```

CI splits them into jobs, so a slow package mirror cannot delay news of a broken commit:

| Job | Covers | Needs system packages |
| --- | --- | --- |
| `rust` | `blkbstr-core`, `blkbstr-daemon`, on Linux **and** Windows | no |
| `frontend` | `npm run build` | no |
| `app` | `blkbstr` (the Tauri crate), on Linux and Windows | yes: the platform webview |
| `packaging` | installer scripts, systemd unit, AppArmor and SELinux profiles | no |

`rust` runs on both platforms rather than just Linux because `blkbstr-daemon` has
`#[cfg(unix)]` / `#[cfg(not(unix))]` branches for the socket; the Windows named-pipe path is
compiled only by the Windows leg. Adding a platform-specific branch without a matching CI leg is
how it goes stale.

Separate workflows check the pull request title (Conventional Commits), review dependency changes
and run CodeQL.

## Commits and pull requests

Use [Conventional Commits](https://www.conventionalcommits.org/). Pull requests are squash merged and
the PR title becomes the commit on `main`, so the title must follow the format; CI checks it.

```
feat: add a log search
fix(daemon): pull the rules down when the engine exits by itself
docs: correct the config format table
```

`feat` bumps the minor version, `fix` the patch, and `feat!` or a `BREAKING CHANGE:` footer the
major (minor while below 1.0). `perf` and `revert` appear in the changelog. `docs` appears too; `ci`,
`chore`, `build`, `refactor`, `test` and `style` do not release on their own.

Keep a pull request to one change; a refactor and a fix in the same PR is two reviews pretending to
be one. Add a test when behaviour changes, and update `docs/` when a user-visible behaviour, setting
or the config format changes.

## Releases

Releases are automatic and nobody edits `CHANGELOG.md` or the version files by hand. Every merge to
`main` updates a release pull request with the next version and the changelog, both generated from
the commit messages. Merging that pull request tags the release, publishes the notes and builds the
Linux packages and `blkbstrd` onto it with a signed provenance attestation. See
[docs/releasing.md](docs/releasing.md).

## What the code should look like

**Non-trivial logic leaves a test behind.** Not a suite - the smallest thing that fails if the
logic breaks. The existing tests in `blkbstr-core` are the model.

**Comments explain what the code cannot.** A confirmed constraint, a non-obvious reason an
"obvious" simplification is wrong, something a reader would be surprised by. Not a restatement of
the line below it.

**The privileged half stays small.** New dependencies in `blkbstr-daemon` need a reason;
everything in that crate runs as root. If a feature can live in the GUI, it lives in the GUI.

**The function registry is honest.** It lists what upstream zapret2 documents, nothing more.
Nothing is rendered from it by name, so a missing entry costs a warning - but an invented entry
offers the user a strategy that cannot work.

## Things that need care

- **Firewall teardown.** Rules must come down on stop, on `Drop`, and on the next startup after a
  kill that skipped both. A change to the engine backend that breaks any of the three strands
  people with a network that half works.
- **Order of operations.** Validate, check with `--intercept=0`, *then* rules, *then* the engine. On the way down,
  rules first. Rules pointing at a queue nothing reads is the state that breaks a machine.
- **Socket permissions.** Anyone who can write the socket can rewrite the firewall.
- **Config names.** They become filenames. `validate()` exists for a reason and both sides call it.
- **Config values.** They become lines in the engine parameter file; a newline injects an arbitrary
  option into a root process. Configs arrive by import and by sync - treat them as untrusted.
- **No telemetry.** Not off-by-default, not opt-in-later. Absent.

## Scope

See the [legal notice](LEGAL.md): changes that add attack tooling, bundle upstream strategy libraries or
send data off the machine are declined.

## Reporting security problems

See [SECURITY.md](SECURITY.md).
