# Platform notes

Per-OS friction that will otherwise be rediscovered painfully, one platform at a time.

## Linux

**Init systems.** The packaging in `packaging/linux/` is systemd-only and `install.sh` says so
rather than producing a broken install on OpenRC or runit. Non-systemd support means a second unit
format and a second way to find the socket; it waits until someone asks.

**Hardening.** The systemd unit drops to `CAP_CHOWN`, `CAP_NET_ADMIN` and `CAP_NET_RAW` —
`CAP_CHOWN` for the one call that puts the socket in the `blkbstr` group, which fails with `EPERM`
even as root once the bounding set is trimmed. AppArmor and SELinux profiles live in
[packaging/linux/](../packaging/linux/README.md) and are loaded by hand, not by the installer.
Either LSM can refuse netlink and leave the daemon reporting `Operation not permitted` from a
process running as root, so that error carries a note naming what could have refused it.

**Distros.** Package manager detection drives engine installation and must be user-overridable;
auto-detection will be wrong on derivatives and on anyone's carefully broken system.

**Firewall backend.** nftables only. Everything goes in a dedicated `inet blkbstr` table, so
teardown is one atomic `nft delete table` that cannot touch another program's rules — in iptables
every program shares the same chains and removal means matching individual rules. Upstream's own
guidance is that iptables is legacy and post-NAT interception is impossible there. An iptables path
is worth adding only for kernels older than 5.15 or `nft` older than 1.0.1, and only if someone
turns up on one.

**Lua.** zapret2 needs LuaJIT 2.1+ or PUC Lua 5.3+ at runtime; the attacks are Lua, not C. A
missing interpreter shows up as an engine that will not start, so onboarding checks for it
explicitly rather than letting the user read a stack trace.

## Windows

**WinDivert.** A signed kernel driver. Loading it needs administrator rights and the signature has
to stay valid — a driver-signing lapse breaks every install at once.

**No firewall rules.** `winws2` builds its own WinDivert filter from the profile filters, so there
is nothing to install or tear down. The whole `firewall.rs` problem does not exist here, which
makes Windows the smaller half of the engine work despite the driver and signing overhead.

**Antivirus.** Anti-censorship tools get flagged as malware routinely: they inject packets, spoof
TTLs and load a network driver, which is a fair description of malware if you are a heuristic. The
false-positive submission process to the major vendors starts *before* the M3 release. Assume at
least one vendor flags it anyway and document the exclusion steps.

**Service.** One UAC-elevated installer step registers the Windows Service. The GUI then talks to
it over a named pipe whose ACL must be restricted to the installing user's group — not yet
implemented, and a release blocker for M3.

## FreeBSD and OpenBSD

`dvtws2` with ipfw (FreeBSD) or pf (OpenBSD), driven by the same config. Cheap to add because
upstream already ships the binary; the work is an rc.d service script and a divert-socket
equivalent of `firewall.rs`.

## macOS

**Not supported, and not planned.** Upstream is explicit:

> MacOS не поддерживается и вряд ли будет по техническим причинам

The manual gives the reason: Apple removed `ipdivert` from the kernel and there is no replacement
packet-interception facility. Supporting macOS would mean writing a Network Extension *and* a
packet manipulator that upstream does not have — a different project, not a milestone of this one.

`Platform::current()` returns `None` there, and macOS is not built in CI — a green build for a
platform we cannot ship is minutes spent proving nothing.

## Distribution

| OS | Channels |
| --- | --- |
| Linux | Flatpak, AUR, deb repo, AppImage |
| Windows | winget, Chocolatey, signed installer |
| FreeBSD, OpenBSD | ports |

Flatpak is the awkward one: a sandboxed app talking to a system service on the host needs a
portal or a socket hole punched through, and the daemon cannot ship inside the Flatpak. It may end
up as "Flatpak GUI, distro package for the daemon", which is two installs and needs to be
explained rather than discovered.

## Signing

Every binary and installer gets signed. Anti-censorship tools are a standard impersonation target,
and "download this build of the tool that gets around censorship" is an effective way to attack
exactly the people this project is for. Users need a way to verify authenticity that does not
depend on trusting a download page.
