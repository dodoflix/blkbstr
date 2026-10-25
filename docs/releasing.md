# Releasing

Releases are made by [Release Please](https://github.com/googleapis/release-please). Nobody edits
`CHANGELOG.md` or a version by hand.

1. Merge pull requests to `main`. Each title (the squash commit) is a
   [Conventional Commit](https://www.conventionalcommits.org/), checked in CI.
2. Release Please keeps one open pull request, "chore(main): release x.y.z", with the next version
   and the changelog it generated from those commits.
3. Merging that pull request tags `vx.y.z` and publishes the GitHub release. The same workflow then
   builds the Linux packages and `blkbstrd` and attaches them to that release, and in parallel
   builds the Windows installer and attaches it. Once both are there, the `publish` job signs
   `SHA256SUMS` over everything attached, attests each file, and publishes the apt and dnf
   repositories to GitHub Pages; the `aur` job pushes `blkbstr-bin` to the AUR
   ([install-linux.md](install-linux.md)).

`feat` bumps the minor version, `fix` and `perf` the patch, and `feat!` or a `BREAKING CHANGE:`
footer the major. While the version is below 1.0 a breaking change bumps the minor. `docs`,
`refactor`, `test`, `ci`, `build`, `chore` and `style` do not release on their own.

Three files carry the version and Release Please updates all of them in the release pull request, so
they cannot drift: `Cargo.toml` (`[workspace.package]`, plus the matching entries in `Cargo.lock`),
`package.json` and `src-tauri/tauri.conf.json`. The list is in `release-please-config.json`.
The `Cargo.lock` paths filter on `@.name.value`: Release Please's TOML parser wraps every value, so
`@.name` matches nothing and the action only logs a warning.

## App updates

The app checks `latest.json` on the latest release (System page, Updates). Release builds sign
the AppImage and the Windows installer with the key in the `TAURI_SIGNING_PRIVATE_KEY` repository
secret, and the `updater-manifest` job publishes `latest.json` once both builds are attached. The
matching public key is `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`; lose the private
key and installed apps can no longer be updated. deb and rpm installs only see that a version
exists. The engine is updated on its own from the System page: it reinstalls the release pinned in
the app, and only when Blockbuster made the install (the `.blkbstr-release` marker).

## Signing and package repositories

Release assets are covered by a GPG-signed `SHA256SUMS` and build provenance attestations, both made
by the `publish` job after every build job has attached its files. Signing the Windows installer
itself is still planned (see "Ongoing" in [plan.md](plan.md)); it belongs in the `windows` job,
before the upload, and the checksums then cover the signed file. Until it lands, SmartScreen warns
on first run and antivirus tools are likelier to flag the installer
([av-submission.md](av-submission.md)). "Download this build of the tool that gets around
censorship" is an effective attack on the people this project is for, so point people only at this
repository's releases page, the Pages repositories or the AUR package.

One-time setup, and why a release fails without it:

- Settings, Pages, Source: GitHub Actions.
- `GPG_PRIVATE_KEY` (`gpg --armor --export-secret-keys <id>`) and `GPG_PASSPHRASE` secrets. The
  public half is served at `key.asc` and signs the apt and dnf metadata and `SHA256SUMS`. Lose the
  key and every user has to import a new one.
- `AUR_SSH_PRIVATE_KEY` secret: an SSH key whose public half is on the maintainer's AUR account.
  The first push creates the package.

The repositories hold only the latest release: each run rebuilds them from that release's `.deb`
and `.rpm`.

## What is not in a release

The engine. Each build pins one upstream zapret2 release in `engine_release.rs` (version and
SHA-256) and installs it on demand, but does not contain it. Bumping the pin means updating both
values and checking `NFQWS2_COMPAT_VER` against the new release.
