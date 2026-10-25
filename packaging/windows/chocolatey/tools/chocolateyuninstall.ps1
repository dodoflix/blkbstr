$ErrorActionPreference = 'Stop'

[array]$key = Get-UninstallRegistryKey -SoftwareName 'Blockbuster*'
if ($key.Count -ne 1) { throw "Expected one Blockbuster uninstall entry, found $($key.Count)" }

Uninstall-ChocolateyPackage -PackageName $env:ChocolateyPackageName -FileType 'exe' `
  -SilentArgs '/S' -File ($key[0].UninstallString -replace '"', '') -ValidExitCodes @(0)
