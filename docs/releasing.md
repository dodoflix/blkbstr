# Releasing

Releases are made by [Release Please](https://github.com/googleapis/release-please). Nobody edits
`CHANGELOG.md` or a version by hand.

1. Merge pull requests to `main`. Each title (the squash commit) is a
   [Conventional Commit](https://www.conventionalcommits.org/), checked in CI.
2. Release Please keeps one open pull request, "chore(main): release x.y.z", with the next version
   and the changelog it generated from those commits.
3. Merging that pull request tags `vx.y.z` and publishes the GitHub release. The same workflow then
   builds the Linux packages and `blkbstrd` and attaches them, with `SHA256SUMS`, to that release.

`feat` bumps the minor version, `fix` and `perf` the patch, and `feat!` or a `BREAKING CHANGE:`
footer the major. While the version is below 1.0 a breaking change bumps the minor. `docs`,
`refactor`, `test`, `ci`, `build`, `chore` and `style` do not release on their own.

Three files carry the version and Release Please updates all of them in the release pull request, so
they cannot drift: `Cargo.toml` (`[workspace.package]`, plus the matching entries in `Cargo.lock`),
`package.json` and `src-tauri/tauri.conf.json`. The list is in `release-please-config.json`.

Linux only for now. Windows joins when there is a Windows service: without one an installer would
produce an app with nothing to talk to.

## Signing

Artefacts carry a [build provenance attestation](https://docs.github.com/actions/security-guides/using-artifact-attestations)
(Sigstore-backed and keyless, so there is no signing key to lose), and each file is tied to the
commit and workflow run that produced it:

```sh
gh attestation verify blkbstrd -R dodoflix/blkbstr
```

That is authenticity of origin. Code signing with a held key, Authenticode on Windows and the AV
vendor submissions that go with it, comes with the Windows work. Until then this is the check to
point people at, because "download this build of the tool that gets around censorship" is an
effective attack on exactly the people this project is for.

## What is not in a release

The engine. zapret2 is installed separately and versioned separately; a version here says nothing
about which upstream version is running.
