# Releasing

A release is a tag. Everything else is [`.github/workflows/release.yml`](../.github/workflows/release.yml).

```sh
# 1. Set the version in all three files that carry one.
#      Cargo.toml            [workspace.package] version
#      package.json          version
#      src-tauri/tauri.conf.json  version
# 2. Commit, merge to main, then tag the merge commit.
git tag v0.1.0
git push origin v0.1.0
```

The workflow refuses the tag if those three disagree, before it spends twenty minutes building.

It then builds the GUI bundles and `blkbstrd` on Linux, writes `SHA256SUMS`, attaches a signed
provenance attestation to each binary, and opens a **draft** release with notes generated from the
commits since the last tag. Publishing is a separate, human click — check the assets first.

Linux only for now. Windows joins at M3: there is no Windows service yet, so an installer would
produce an app with nothing to talk to.

## Signing

Artefacts carry a [build provenance attestation](https://docs.github.com/actions/security-guides/using-artifact-attestations)
— Sigstore-backed and keyless, so there is no signing key to lose, and each file is tied to the
commit and workflow run that produced it:

```sh
gh attestation verify blkbstrd -R dodoflix/blkbstr
```

That is authenticity of *origin*. Code signing with a held key — Authenticode on Windows, and the
AV vendor submissions that go with it — is M3. Until then, this is the check to point people at,
because "download this build of the tool that gets around censorship" is an effective attack on
exactly the people this project is for.

## What is not in a release

The engine. zapret2 is installed separately and versioned separately; a version here says nothing
about which upstream version is running.
