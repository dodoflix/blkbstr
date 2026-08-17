#!/bin/sh
# Installs blkbstrd as a systemd service and puts the invoking user in the blkbstr group.
# Run as root:  sudo packaging/linux/install.sh [path-to-blkbstrd]
set -eu

BIN=${1:-target/release/blkbstrd}
LIBEXEC=/usr/local/libexec/blkbstrd
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

[ "$(id -u)" -eq 0 ] || { echo "run as root" >&2; exit 1; }
[ -x "$BIN" ] || { echo "no blkbstrd binary at $BIN (cargo build --release -p blkbstr-daemon)" >&2; exit 1; }
command -v systemctl >/dev/null || { echo "this installer is systemd-only; see docs/platform-notes.md" >&2; exit 1; }

# The group is the access control: members can drive the engine without further prompts.
groupadd -f blkbstr
GID=$(getent group blkbstr | cut -d: -f3)

# SUDO_USER is the human who ran sudo; without it there is nobody to add.
if [ -n "${SUDO_USER:-}" ]; then
    usermod -aG blkbstr "$SUDO_USER"
    echo "added $SUDO_USER to the blkbstr group — log out and back in for it to take effect"
else
    echo "warning: no SUDO_USER; add your account with: usermod -aG blkbstr <user>" >&2
fi

install -Dm755 "$BIN" "$LIBEXEC"
install -Dm644 "$HERE/blkbstrd.service" /etc/systemd/system/blkbstrd.service
install -Dm644 "$HERE/dev.blkbstr.manage-service.policy" \
    /usr/share/polkit-1/actions/dev.blkbstr.manage-service.policy
install -d -m 755 /etc/blkbstr
printf 'BLKBSTR_GID=%s\n' "$GID" > /etc/blkbstr/daemon.env

systemctl daemon-reload
systemctl enable --now blkbstrd.service
systemctl --no-pager --lines=0 status blkbstrd.service
