#!/bin/sh
# Installs blkbstrd as a systemd service and puts the invoking user in the blkbstr group.
# Run as root:  sudo packaging/linux/install.sh [path-to-blkbstrd]
# For a source checkout. The packaged app does the same from its own window, through
# `pkexec blkbstrd install-service`.
set -eu

BIN=${1:-target/release/blkbstrd}

[ "$(id -u)" -eq 0 ] || { echo "run as root" >&2; exit 1; }
[ -x "$BIN" ] || { echo "no blkbstrd binary at $BIN (cargo build --release -p blkbstr-daemon)" >&2; exit 1; }

exec "$BIN" install-service
