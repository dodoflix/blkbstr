# Builds blkbstrd and puts it where Tauri's externalBin expects it, so the installer carries it
# next to the app. Needed before any build of the GUI crate on Windows, including `cargo check`.
param([ValidateSet('debug', 'release')][string]$Profile = 'debug')
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')

if ($Profile -eq 'release') { cargo build --release -p blkbstr-daemon } else { cargo build -p blkbstr-daemon }
if ($LASTEXITCODE -ne 0) { throw 'building blkbstrd failed' }

$bin = 'src-tauri\binaries'
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item "target\$Profile\blkbstrd.exe" "$bin\blkbstrd-x86_64-pc-windows-msvc.exe" -Force
