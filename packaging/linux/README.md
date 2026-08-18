# Linux packaging

`install.sh` puts the daemon at `/usr/local/libexec/blkbstrd`, installs the systemd unit and the
polkit action, creates the `blkbstr` group and adds the invoking user to it. `uninstall.sh` removes
all of it.

## Confinement

The daemon runs unconfined by default. Both profiles below are optional, and neither is loaded by
`install.sh`: confining the process that owns your firewall is a decision for whoever runs the
machine, not for an installer.

Their value is not only containment. On a system where an LSM is already enforcing, a daemon with
no profile can be refused netlink and report it as `Operation not permitted` — from a process
running as root, which sends people looking at file modes. The daemon appends a note to any such
error saying what could actually have refused it.

### AppArmor

```sh
sudo cp apparmor/usr.local.libexec.blkbstrd /etc/apparmor.d/
sudo apparmor_parser -r -W /etc/apparmor.d/usr.local.libexec.blkbstrd
sudo systemctl restart blkbstrd
```

Remove it with `apparmor_parser -R`, then delete the file — a profile left in `/etc/apparmor.d` is
loaded again at boot.

`nft` and `ip` run inside the profile; the engine runs under its own profile if the distribution
ships one and unconfined otherwise, because it reads hostlists and ipsets from wherever your
config points and no profile written here could enumerate them.

### SELinux

```sh
cd selinux
make -f /usr/share/selinux/devel/Makefile blkbstr.pp   # needs selinux-policy-devel
sudo semodule -i blkbstr.pp
sudo restorecon -RvF /usr/local/libexec/blkbstrd /var/lib/blkbstr /var/log/blkbstr /run/blkbstr
sudo systemctl restart blkbstrd
```

Remove it with `sudo semodule -r blkbstr`.

The module is built and linked against Fedora's targeted policy in CI, and the transitions it
relies on are asserted there. It has not been run under an enforcing kernel, so treat a denial as
a bug worth reporting rather than something to work around:

```sh
sudo ausearch -m avc -ts recent | audit2allow -R
```

`blkbstrd.if` exports `blkbstrd_stream_connect()` and `blkbstrd_read_log()` for anyone running the
GUI in a confined domain. On a stock desktop the GUI is `unconfined_t` and already has both.

## Non-systemd init

`install.sh` refuses rather than producing a broken install. Supporting OpenRC or runit means a
second unit format and a second way to find the socket; it waits until someone asks.
