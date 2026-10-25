# Authenticode-signs one file with the certificate in the WINDOWS_CERT_PFX_BASE64 secret.
# Refuses to run without it: an unsigned build of an anti-censorship tool is an impersonation
# target, so a release must fail rather than ship one.
param([Parameter(Mandatory)][string]$File)
$ErrorActionPreference = 'Stop'

if (-not $env:WINDOWS_CERT_PFX_BASE64) {
    throw 'WINDOWS_CERT_PFX_BASE64 is not set; refusing to produce an unsigned binary'
}
$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" |
    Sort-Object FullName | Select-Object -Last 1
if (-not $signtool) { throw 'signtool.exe not found; the Windows SDK is missing' }

$pfx = Join-Path ([IO.Path]::GetTempPath()) "blkbstr-signing-$PID.pfx"
try {
    [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:WINDOWS_CERT_PFX_BASE64))
    & $signtool.FullName sign /f $pfx /p $env:WINDOWS_CERT_PASSWORD /fd sha256 `
        /tr http://timestamp.digicert.com /td sha256 $File
    if ($LASTEXITCODE -ne 0) { throw "signtool failed on $File" }
} finally {
    Remove-Item $pfx -Force -ErrorAction SilentlyContinue
}
