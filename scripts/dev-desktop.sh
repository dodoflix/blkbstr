#!/bin/sh
# Gives `npm run tauri dev` a taskbar icon on Wayland.
#
# Wayland has no per-window icons: the compositor takes the window's app id ("blkbstr") and looks for
# a desktop entry of that name. Packaged builds ship one; a dev run does not, so the taskbar shows a
# generic icon whatever the window sets. This installs a hidden entry that points at the repo's icon
# and binary. It is not a cache problem, but the desktop database does need rebuilding once.
#
#   scripts/dev-desktop.sh            install
#   scripts/dev-desktop.sh --remove   uninstall
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
DIR=${XDG_DATA_HOME:-$HOME/.local/share}/applications
FILE=$DIR/blkbstr.desktop

refresh() {
    for tool in kbuildsycoca6 kbuildsycoca5 update-desktop-database; do
        if command -v "$tool" >/dev/null 2>&1; then
            "$tool" >/dev/null 2>&1 || true
            return
        fi
    done
}

if [ "${1:-}" = "--remove" ]; then
    rm -f "$FILE"
    refresh
    echo "removed $FILE"
    exit 0
fi

mkdir -p "$DIR"
cat >"$FILE" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Blockbuster (dev)
Exec=$ROOT/target/debug/blkbstr
Icon=$ROOT/src-tauri/icons/128x128.png
StartupWMClass=blkbstr
NoDisplay=true
DESKTOP
refresh
echo "installed $FILE; restart npm run tauri dev"
