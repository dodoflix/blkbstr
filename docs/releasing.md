# Releasing

Releases are made by [Release Please](https://github.com/googleapis/release-please). Nobody edits
`CHANGELOG.md` or a version by hand.

1. Merge pull requests to `main`. Each title (the squash commit) is a
   [Conventional Commit](https://www.conventionalcommits.org/), checked in CI.
2. Release Please keeps one open pull request, "chore(main): release x.y.z", with the next version
   and the changelog it generated from those commits.
3. Merging that pull request tags `vx.y.z` and publishes the GitHub release. The same workflow then
   builds the Linux packages and `blkbstrd` and attaches them, with `SHA256SUMS`, to that release,
   and in parallel the signed Windows installer with `SHA256SUMS-windows.txt` and the filled-in
   winget and Chocolatey packages (`windows-packages.zip`).

`feat` bumps the minor version, `fix` and `perf` the patch, and `feat!` or a `BREAKING CHANGE:`
footer the major. While the version is below 1.0 a breaking change bumps the minor. `docs`,
`refactor`, `test`, `ci`, `build`, `chore` and `style` do not release on their own.

Three files carry the version and Release Please updates all of them in the release pull request, so
they cannot drift: `Cargo.toml` (`[workspace.package]`, plus the matching entries in `Cargo.lock`),
`package.json` and `src-tauri/tauri.conf.json`. The list is in `release-please-config.json`.

## Signing

Artefacts carry a [build provenance attestation](https://docs.github.com/actions/security-guides/using-artifact-attestations)
(Sigstore-backed and keyless, so there is no signing key to lose), and each file is tied to the
commit and workflow run that produced it:

```sh
gh attestation verify blkbstrd -R dodoflix/blkbstr
```

That is authenticity of origin, and it is the check to point people at, because "download this
build of the tool that gets around censorship" is an effective attack on exactly the people this
project is for.

Windows adds Authenticode on top, but **temporarily** there is no certificate, so the release job
ships an unsigned installer (it logs a warning) and the release notes say so. Users are pointed at
the attestation and `SHA256SUMS-windows.txt` instead. When a certificate exists, set these two
repository secrets and the same job signs everything, then make the job fail without them (see the
`TEMPORARY` comment in `release.yml`):

| Secret | What |
| --- | --- |
| `WINDOWS_CERT_PFX_BASE64` | The code-signing certificate and key as a `.pfx`, base64-encoded |
| `WINDOWS_CERT_PASSWORD` | Its password |

`packaging/windows/sign.ps1` signs the daemon before it is bundled, and Tauri signs the app and the
installer through it. With the secrets set, the job also checks every `.exe` it ships reports a
valid signature.

### Before the first Windows release

Things only a person can do, in this order:

1. Once available: get a code-signing certificate and set the two secrets. An EV certificate skips
   most of SmartScreen's reputation wait; a standard one works but starts untrusted.
2. Build a pre-release, run it on a real Windows 10 or 11 machine and go through the wizard once:
   install the helper, install the engine, find a strategy, reboot, check it came back.
3. Submit that build to the antivirus vendors, per [av-submission.md](av-submission.md).
4. Publish the packages from `windows-packages.zip` by hand, per
   [packaging/windows/README.md](../packaging/windows/README.md). The release workflow does not
   publish them, because there are no winget or Chocolatey accounts yet.

## What is not in a release

The engine. Each build pins one upstream zapret2 release in `engine_release.rs` (version and
SHA-256) and installs it on demand, but does not contain it. Bumping the pin means updating both
values and checking `NFQWS2_COMPAT_VER` against the new release.
