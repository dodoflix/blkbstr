#!/bin/sh
# Removes everything install.sh put on the system. Run as root.
# Leaves per-user configs under ~/.config/blkbstr alone.
set -eu

[ "$(id -u)" -eq 0 ] || { echo "run as root" >&2; exit 1; }

systemctl disable --now blkbstrd.service 2>/dev/null || true
rm -f /etc/systemd/system/blkbstrd.service \
      /usr/share/polkit-1/actions/dev.blkbstr.manage-service.policy \
      /usr/local/libexec/blkbstrd \
      /etc/blkbstr/daemon.env
rmdir /etc/blkbstr 2>/dev/null || true
systemctl daemon-reload

# The group is deliberately kept. Deleting it means the next install gets a fresh GID, which the
# already-running desktop session cannot pick up, so every reinstall would cost the user a logout.
# On its own it grants nothing: the socket it guards is gone with the service.
rm -rf /var/lib/blkbstr
echo "removed. logs remain in /var/log/blkbstr; per-user configs in ~/.config/blkbstr"
echo "the blkbstr group is kept so a reinstall does not need a logout; drop it with: groupdel blkbstr"
