# Platform notes

Per-OS friction that will otherwise be rediscovered painfully, one platform at a time.

## Linux

**Init systems.** The packaging in `packaging/linux/` is systemd-only and `install.sh` says so
rather than producing a broken install on OpenRC or runit. Non-systemd support means a second unit
format and a second way to find the socket; it waits until someone asks.

**Hardening.** The systemd unit drops to `CAP_CHOWN`, `CAP_NET_ADMIN` and `CAP_NET_RAW` -
`CAP_CHOWN` for the one call that puts the socket in the `blkbstr` group, which fails with `EPERM`
even as root once the bounding set is trimmed. AppArmor and SELinux profiles live in
[packaging/linux/](../packaging/linux/README.md) and are loaded by hand, not by the installer.
Either LSM can refuse netlink and leave the daemon reporting `Operation not permitted` from a
process running as root, so that error carries a note naming what could have refused it.

Two systemd hardening options cannot be used, and both fail in ways that do not name themselves.
`MemoryDenyWriteExecute=yes` kills LuaJIT - the attacks are Lua, and the engine exits with
`runtime code generation failed, restricted kernel?` - and the seccomp filter is inherited by
children, so it cannot be kept for the daemon alone. `ProtectHome=yes` hides `/home`, so a config
pointing `hostlist=` at a file the user can see is rejected as missing; the unit uses `read-only`
instead, and SELinux gates the same thing behind `blkbstrd_read_user_content`.

**Engine install.** The wizard installs a pinned upstream release into `/opt/zapret2` rather than
going through a package manager, so it does not depend on which distro this is or on anyone having
packaged zapret2. It needs `curl` and `pkexec` (polkit) and a CPU upstream ships a binary for
(x86, x86_64, arm, arm64, riscv64). The package manager is still detected from `ID`/`ID_LIKE` and
shown in the advanced view, but nothing acts on it.

**Firewall backend.** nftables only. Everything goes in a dedicated `inet blkbstr` table, so
teardown is one atomic `nft delete table` that cannot touch another program's rules - in iptables
every program shares the same chains and removal means matching individual rules. Upstream's own
guidance is that iptables is legacy and post-NAT interception is impossible there. An iptables path
is worth adding only for kernels older than 5.15 or `nft` older than 1.0.1, and only if someone
turns up on one.

**Lua.** zapret2 needs LuaJIT 2.1+ or PUC Lua 5.3+ at runtime; the attacks are Lua, not C. A
missing interpreter shows up as an engine that will not start, so onboarding checks for it
explicitly rather than letting the user read a stack trace.

## Windows

**WinDivert.** A signed kernel driver. Loading it needs administrator rights and the signature has
to stay valid - a driver-signing lapse breaks every install at once. It ships in upstream's zip next
to `winws2.exe`, and the wizard installs both into `%ProgramFiles%\zapret2`.

**No firewall rules.** `winws2` builds its own WinDivert filter from `--wf-tcp-out` and
`--wf-udp-out`, which the renderer writes from the ports the enabled strategies select. There is
nothing to install or tear down, so `firewall.rs` is Linux-only in practice. Only outgoing ports
are named: SYN+ACK, FIN and RST come back on their own, and WinDivert cannot limit interception to
a flow's first packets the way conntrack does, so intercepting every incoming segment of a download
is CPU spent for nothing. The exception is the auto host list (which also adds a raw filter that
skips the daemon's own probes, marked by their TTL, so the periodic check never goes through the
strategy): its failure detector reads resets
and retransmissions, so while it is in use the renderer adds `--wf-tcp-in` and `--wf-udp-in` for the
same ports, and Windows pays that CPU cost.

**Stopping restores the machine.** The engine's filter dies with its process, and the daemon puts
the engine in a job object that kills it when the daemon goes, however that happens. Without that a
killed daemon leaves `winws2` running, and the next start is refused as a duplicate of the same
WinDivert filter.

**A Cygwin program.** `winws2.exe` needs `cygwin1.dll` beside it, and it reads paths its own way, so
every path in a parameter file is written with forward slashes (`C:/ProgramData/...`). After
starting it lowers its own integrity level; it creates and labels its log file first, which is why
the log directory only has to be writable by the service. LuaJIT is linked in, so the wizard does not
look for a Lua runtime or nftables on Windows.

**Antivirus.** Anti-censorship tools get flagged as malware routinely: they inject packets, spoof
TTLs and load a network driver, which is a fair description of malware if you are a heuristic. The
submission process and the exclusion steps for users are in [av-submission.md](av-submission.md).
Assume at least one vendor flags it anyway. Releases are not signed, which makes that likelier
and means SmartScreen warns on first run.

**Service.** `blkbstrd install-service --allow-sid <SID>`, run elevated by the GUI in one UAC
prompt, registers an auto-start service as SYSTEM, restarts it after a crash, and does three things
that decide who can reach SYSTEM:

- It refuses unless the binary is under Program Files, where only administrators can replace it.
- It resets the owner and every entry on `C:\ProgramData\blkbstr`, so a directory a user created
  there first cannot feed the service a config. Users may read it; the GUI tails logs from there.
- It sets the service's own access list so the installing user can start and stop the helper
  without a prompt, and nothing more. Change-config is withheld on purpose: it lets its holder
  repoint the service at any binary.

The named pipe admits SYSTEM and that one user, and refuses remote clients. Changing the start type
(the "Start at boot" switch) needs administrator rights and prompts each time.

**Uninstall.** The installer's hooks stop the service before an upgrade replaces `blkbstrd.exe`,
remove it on uninstall and delete `C:\ProgramData\blkbstr`. The engine in Program Files stays: it
was installed by the wizard, not by the installer.

**Antivirus.** Windows Security flags the WinDivert driver that ships with the engine and removes it
as it is written. The engine install therefore excludes `C:\Program Files\zapret2` from Defender's
scans first (`Add-MpPreference`), and uninstalling Blockbuster removes that exclusion when the
engine there is one the wizard installed. With another antivirus, or Tamper Protection on, the
exclusion cannot be set and the install says the file was blocked.

**Windows on ARM.** Not offered. Upstream's answer is the x64 build with a test-signed driver and
Test Signing mode switched on, which is not something to do behind a button.

## FreeBSD and OpenBSD

`dvtws2` with ipfw (FreeBSD) or pf (OpenBSD), driven by the same config. Cheap to add because
upstream already ships the binary; the work is an rc.d service script and a divert-socket
equivalent of `firewall.rs`.

## macOS

**Not supported, and not planned.** Upstream is explicit:

> MacOS не поддерживается и вряд ли будет по техническим причинам

The manual gives the reason: Apple removed `ipdivert` from the kernel and there is no replacement
packet-interception facility. Supporting macOS would mean writing a Network Extension *and* a
packet manipulator that upstream does not have - a different project, not a milestone of this one.

`Platform::current()` returns `None` there, and macOS is not built in CI - a green build for a
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
