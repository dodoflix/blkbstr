# Releasing

Releases are made by [Release Please](https://github.com/googleapis/release-please). Nobody edits
`CHANGELOG.md` or a version by hand.

1. Merge pull requests to `main`. Each title (the squash commit) is a
   [Conventional Commit](https://www.conventionalcommits.org/), checked in CI.
2. Release Please keeps one open pull request, "chore(main): release x.y.z", with the next version
   and the changelog it generated from those commits.
3. Merging that pull request tags `vx.y.z` and publishes the GitHub release. The same workflow then
   builds the Linux packages and `blkbstrd` and attaches them to that release, and in parallel
   builds the Windows installer and attaches it.

`feat` bumps the minor version, `fix` and `perf` the patch, and `feat!` or a `BREAKING CHANGE:`
footer the major. While the version is below 1.0 a breaking change bumps the minor. `docs`,
`refactor`, `test`, `ci`, `build`, `chore` and `style` do not release on their own.

Three files carry the version and Release Please updates all of them in the release pull request, so
they cannot drift: `Cargo.toml` (`[workspace.package]`, plus the matching entries in `Cargo.lock`),
`package.json` and `src-tauri/tauri.conf.json`. The list is in `release-please-config.json`.

## App updates

The app checks `latest.json` on the latest release (System page, Updates). Release builds sign
the AppImage and the Windows installer with the key in the `TAURI_SIGNING_PRIVATE_KEY` repository
secret, and the `updater-manifest` job publishes `latest.json` once both builds are attached. The
matching public key is `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`; lose the private
key and installed apps can no longer be updated. deb and rpm installs only see that a version
exists. The engine is updated on its own from the System page: it reinstalls the release pinned in
the app, and only when Blockbuster made the install (the `.blkbstr-release` marker).

## Signing

Nothing in a release is signed or attested yet, and there are no checksums. That matters more than
usual here: "download this build of the tool that gets around censorship" is an effective attack on
exactly the people this project is for, so signing, checksums and build provenance attestations
are planned (see "Ongoing" in [plan.md](plan.md)). Until then, point people only at this
repository's releases page. On Windows, SmartScreen warns on first run and antivirus tools are
likelier to flag the installer ([av-submission.md](av-submission.md)).

## What is not in a release

The engine. Each build pins one upstream zapret2 release in `engine_release.rs` (version and
SHA-256) and installs it on demand, but does not contain it. Bumping the pin means updating both
values and checking `NFQWS2_COMPAT_VER` against the new release.
