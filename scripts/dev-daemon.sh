#!/bin/sh
# Builds blkbstrd and runs it from target/debug in the foreground.
#
# --socket-group-gid is your own primary group, not blkbstr: a session always has its own primary
# group, so a dev daemon needs no group membership and no logout. The installed service, which uses
# the blkbstr group, is stopped first because both want the same socket path.
#
# Ctrl-C stops it. Bring the real one back with: sudo systemctl start blkbstrd
set -eu

cd "$(dirname "$0")/.."
cargo build -p blkbstr-daemon

sudo systemctl stop blkbstrd.service 2>/dev/null || true
exec sudo target/debug/blkbstrd --socket-group-gid "$(id -g)" "$@"
