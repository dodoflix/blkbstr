# Builds blkbstrd and puts it where Tauri's externalBin expects it, so the installer carries it
# next to the app. Needed before any build of the GUI crate on Windows, including `cargo check`.
param([ValidateSet('debug', 'release')][string]$Profile = 'debug')
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')

# The C runtime is linked in: a clean Windows has no VC++ redistributable, and without it blkbstrd
# dies with 0xC0000135 before the engine install or the service can do anything. The GUI already
# links it through tauri-build, so the flag is set for this build only and the GUI's is unchanged.
$rustflags = $env:RUSTFLAGS
$env:RUSTFLAGS = "$rustflags -C target-feature=+crt-static".Trim()
try {
    if ($Profile -eq 'release') { cargo build --release -p blkbstr-daemon } else { cargo build -p blkbstr-daemon }
} finally {
    $env:RUSTFLAGS = $rustflags
}
if ($LASTEXITCODE -ne 0) { throw 'building blkbstrd failed' }

$bin = 'src-tauri\binaries'
New-Item -ItemType Directory -Force $bin | Out-Null
Copy-Item "target\$Profile\blkbstrd.exe" "$bin\blkbstrd-x86_64-pc-windows-msvc.exe" -Force
