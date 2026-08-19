#!/bin/sh
# Builds blkbstrd and runs it from target/debug in the foreground. With --watch, rebuilds and
# restarts it whenever a .rs file changes.
#
# --socket-group-gid is your own primary group, not blkbstr: a session always has its own primary
# group, so a dev daemon needs no group membership and no logout. The installed service, which uses
# the blkbstr group, is stopped first because both want the same socket path.
#
# Ctrl-C stops it. Bring the real one back with: sudo systemctl start blkbstrd
set -eu

cd "$(dirname "$0")/.."
BIN=target/debug/blkbstrd
PIDFILE=${TMPDIR:-/tmp}/blkbstr-dev-daemon.pid
GID=$(id -g)

build() { cargo build -p blkbstr-daemon; }

# sudo forks, so its own pid is not the daemon's. The shell it execs writes the pid we can signal.
start() {
    ${SUDO-sudo} sh -c "echo \$\$ > '$PIDFILE'; exec $BIN --socket-group-gid $GID $*" &
}

stop() {
    [ -f "$PIDFILE" ] || return 0
    ${SUDO-sudo} kill "$(cat "$PIDFILE")" 2>/dev/null || true
    ${SUDO-sudo} rm -f "$PIDFILE"
    wait 2>/dev/null || true
}

# ponytail: polls mtimes once a second. inotify would need a tool that is not installed, and a
# second of latency is invisible next to the build that follows it.
wait_for_change() {
    stamp=$(mktemp)
    while [ -z "$(find crates src-tauri/src -name '*.rs' -newer "$stamp" -print -quit)" ]; do
        sleep 1
    done
    rm -f "$stamp"
}

${SUDO-sudo} systemctl stop blkbstrd.service 2>/dev/null || true

if [ "${1:-}" != "--watch" ]; then
    build
    exec ${SUDO-sudo} $BIN --socket-group-gid "$GID" "$@"
fi
shift

trap 'stop; exit 0' INT TERM
while :; do
    if build; then
        start "$@"
    else
        echo "build failed; waiting for the next change" >&2
    fi
    wait_for_change
    stop
done
