#!/bin/sh
# Fills the winget and Chocolatey templates for one release.
# usage: render.sh <version> <installer-url> <installer-sha256> <out-dir>
set -eu
[ $# -eq 4 ] || { echo "usage: $0 <version> <installer-url> <installer-sha256> <out-dir>" >&2; exit 2; }
version=$1 url=$2 sha=$3 out=$4
case $sha in *[!0-9A-Fa-f]* | '') echo "not a SHA-256: $sha" >&2; exit 2 ;; esac
[ ${#sha} -eq 64 ] || { echo "not a SHA-256: $sha" >&2; exit 2; }

here=$(cd "$(dirname "$0")" && pwd)
rm -rf "$out"
mkdir -p "$out"
cp -R "$here/winget" "$out/winget"
cp -R "$here/chocolatey" "$out/chocolatey"

# `|` as the delimiter: the URL is full of slashes.
find "$out" -type f | while read -r f; do
  sed -i.bak -e "s|__VERSION__|$version|g" -e "s|__URL__|$url|g" -e "s|__SHA256__|$sha|g" "$f"
  rm -f "$f.bak"
done
if grep -rn '__[A-Z0-9]*__' "$out"; then echo "unfilled placeholder" >&2; exit 1; fi
