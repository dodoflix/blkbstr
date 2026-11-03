//! Encrypted DNS. Turkey answers lookups for Discord with a fake address, so on Windows 11 the
//! machine's DNS is moved to Cloudflare over HTTPS (built in since build 22000). Windows 10 has no
//! system DoH, and plain DNS to 1.1.1.1 is intercepted as readily as the ISP's, so it stays out.
//!
//! Every adapter that is up and has a default gateway is switched; its old DNS settings and the
//! DoH entries touched are written to `dns-backup.json` first, and that file existing is what
//! "active" means. Restoring replays the file and deletes it. The scripts are fixed text: the only
//! thing passed in is the backup path, through the environment.

use anyhow::{bail, Result};
use blkbstr_core::paths;
use blkbstr_core::protocol::DnsStatus;
use blkbstr_core::reachability::{CONTROL_HOST, PORT};
use std::path::PathBuf;
use std::sync::Mutex;

/// Encrypt and restore are separate requests on separate connections and must not interleave.
static BUSY: Mutex<()> = Mutex::new(());

fn backup_path() -> PathBuf {
    paths::state_dir().join("dns-backup.json")
}

pub fn status() -> DnsStatus {
    let reason = unsupported();
    DnsStatus {
        supported: reason.is_none(),
        active: reason.is_none() && backup_path().exists(),
        reason,
    }
}

pub fn encrypt() -> Result<()> {
    let _busy = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(why) = unsupported() {
        bail!(why);
    }
    // Already switched: the backup on disk is the original one and must stay that way.
    if backup_path().exists() {
        return Ok(());
    }
    if let Err(e) = sys::run(sys::BACKUP) {
        let _ = std::fs::remove_file(backup_path());
        return Err(e);
    }
    // Fallback is off, so a blocked DoH would take the machine offline: prove it resolves or undo.
    if let Err(e) = sys::run(sys::APPLY).and_then(|()| resolves()) {
        return Err(match sys::run(sys::RESTORE) {
            Ok(()) => e,
            Err(undo) => e.context(format!("and putting DNS back failed too: {undo:#}")),
        });
    }
    tracing::info!("DNS switched to Cloudflare over HTTPS");
    Ok(())
}

/// Looks up the control host through the system resolver, once more after a second because the
/// resolver may need a moment to pick up new servers.
fn resolves() -> Result<()> {
    use std::net::ToSocketAddrs;
    for attempt in 0..2 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        let (tx, rx) = std::sync::mpsc::channel();
        // A lookup cannot be cancelled, so a stuck one is left behind rather than waited for.
        std::thread::spawn(move || {
            let _ = tx.send((CONTROL_HOST, PORT).to_socket_addrs().map(|_| ()));
        });
        if let Ok(Ok(())) = rx.recv_timeout(std::time::Duration::from_secs(5)) {
            return Ok(());
        }
    }
    bail!("Encrypted DNS did not answer on this network, so your DNS settings were put back.")
}

pub fn restore() -> Result<()> {
    let _busy = BUSY.lock().unwrap_or_else(|e| e.into_inner());
    if !backup_path().exists() {
        return Ok(());
    }
    sys::run(sys::RESTORE)?;
    tracing::info!("DNS put back");
    Ok(())
}

#[cfg(not(windows))]
fn unsupported() -> Option<String> {
    Some("Only available on Windows for now".into())
}

#[cfg(not(windows))]
mod sys {
    pub const BACKUP: &str = "";
    pub const APPLY: &str = "";
    pub const RESTORE: &str = "";

    pub fn run(_: &str) -> anyhow::Result<()> {
        anyhow::bail!("Only available on Windows for now")
    }
}

#[cfg(windows)]
fn unsupported() -> Option<String> {
    use blkbstr_core::process;
    // `reg.exe` rather than PowerShell: this runs on every look at the wizard.
    let out = process::command("reg.exe")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        ])
        .args(["/v", "CurrentBuildNumber"])
        .output()
        .ok()?;
    let build: u32 = String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .last()
        .and_then(|b| b.parse().ok())
        .unwrap_or(0);
    (build < 22000).then(|| "Encrypted DNS needs Windows 11".into())
}

#[cfg(windows)]
mod sys {
    use anyhow::{bail, Context, Result};
    use blkbstr_core::process;

    const PRELUDE: &str = r"
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$file = $env:BLKBSTR_DNS_BACKUP
$cf4 = '1.1.1.1', '1.0.0.1'
$cf6 = '2606:4700:4700::1111', '2606:4700:4700::1001'
$all = $cf4 + $cf6
$tpl = 'https://cloudflare-dns.com/dns-query'
$ifr = 'SYSTEM\CurrentControlSet\Services\Dnscache\InterfaceSpecificParameters'
$hklm = [Microsoft.Win32.Registry]::LocalMachine
";

    /// Records what is about to change. Nothing is changed here.
    pub const BACKUP: &str = r"
if ([Environment]::OSVersion.Version.Build -lt 22000) { throw 'Encrypted DNS needs Windows 11' }
$adapters = @()
foreach ($c in Get-NetIPConfiguration) {
  # IPv6 only counts with an address that can leave the LAN (not fe80::/10 link-local or the deprecated fec0::/10 site-local): a gateway and nothing behind it makes every first lookup wait for the v6 server to time out.
  $v6 = [bool]$c.IPv6DefaultGateway -and [bool]($c.IPv6Address | Where-Object { $_.IPAddress -notmatch '^fe[89a-f]' })
  $gw = @{ 4 = [bool]$c.IPv4DefaultGateway; 6 = $v6 }
  if ($c.NetAdapter.Status -ne 'Up' -or -not ($gw[4] -or $gw[6])) { continue }
  $g = $c.NetAdapter.InterfaceGuid
  $families = @()
  foreach ($f in 4, 6) {
    $tcp = if ($f -eq 6) { 'Tcpip6' } else { 'Tcpip' }
    $ns = (Get-ItemProperty -Path ('HKLM:\SYSTEM\CurrentControlSet\Services\' + $tcp + '\Parameters\Interfaces\' + $g) -Name NameServer -ErrorAction SilentlyContinue).NameServer
    $servers = @(Get-DnsClientServerAddress -InterfaceIndex $c.InterfaceIndex -AddressFamily ('IPv' + $f) | Select-Object -ExpandProperty ServerAddresses)
    $families += @{ family = $f; change = $gw[$f]; static = [bool]$ns; servers = $servers }
  }
  $reg = @()
  foreach ($ip in $all) {
    $sub = if ($ip.Contains(':')) { 'Doh6' } else { 'Doh' }
    $k = $hklm.OpenSubKey($ifr + '\' + $g + '\DohInterfaceSettings\' + $sub + '\' + $ip)
    if ($k) { $reg += @{ sub = $sub; ip = $ip; template = $k.GetValue('DohTemplate'); flags = $k.GetValue('DohFlags') }; $k.Close() }
  }
  $adapters += @{ guid = $g; index = $c.InterfaceIndex; families = $families; reg = $reg; hadDoh = (Test-Path -LiteralPath ('HKLM:\' + $ifr + '\' + $g + '\DohInterfaceSettings')) }
}
if (-not $adapters) { throw 'No active network connection found' }
$doh = @()
foreach ($ip in $all) {
  $d = Get-DnsClientDohServerAddress -ServerAddress $ip -ErrorAction SilentlyContinue
  if ($d) { $doh += @{ ip = $ip; template = $d.DohTemplate; fallback = [bool]$d.AllowFallbackToUdp; upgrade = [bool]$d.AutoUpgrade } }
}
New-Item -ItemType Directory -Force -Path (Split-Path $file) | Out-Null
ConvertTo-Json -InputObject @{ adapters = $adapters; doh = $doh } -Depth 6 | Set-Content -Path $file -Encoding UTF8
";

    /// Switches the adapters and families the backup marks as changed.
    pub const APPLY: &str = r"
$b = Get-Content -Raw -Path $file | ConvertFrom-Json
foreach ($ip in $all) {
  if (Get-DnsClientDohServerAddress -ServerAddress $ip -ErrorAction SilentlyContinue) {
    Set-DnsClientDohServerAddress -ServerAddress $ip -DohTemplate $tpl -AllowFallbackToUdp $false -AutoUpgrade $true
  } else {
    Add-DnsClientDohServerAddress -ServerAddress $ip -DohTemplate $tpl -AllowFallbackToUdp $false -AutoUpgrade $true
  }
}
foreach ($a in $b.adapters) {
  foreach ($f in $a.families) {
    if (-not $f.change) { continue }
    $six = $f.family -eq 6
    $ips = if ($six) { $cf6 } else { $cf4 }
    $sub = if ($six) { 'Doh6' } else { 'Doh' }
    foreach ($ip in $ips) {
      # What Settings writes for 'On (automatic template)' with fallback off: flags 0x11.
      $k = $hklm.CreateSubKey($ifr + '\' + $a.guid + '\DohInterfaceSettings\' + $sub + '\' + $ip)
      $k.SetValue('DohTemplate', $tpl, 'String')
      $k.SetValue('DohFlags', [long]17, 'QWord')
      $k.Close()
    }
    Set-DnsClientServerAddress -InterfaceIndex $a.index -ServerAddresses $ips
  }
}
Clear-DnsClientCache
";

    /// Puts everything back from the backup and deletes it. Anything that fails keeps the backup,
    /// so a retry still knows the original state.
    pub const RESTORE: &str = r"
$b = Get-Content -Raw -Path $file | ConvertFrom-Json
$errors = @()
foreach ($a in $b.adapters) {
  try {
    $n = Get-NetAdapter | Where-Object { $_.InterfaceGuid -eq $a.guid }
    if (-not $n) { continue }
    $i = $n.ifIndex
    # Resetting clears both families, so a static list on the other one is set again after it.
    $reset = $a.families | Where-Object { $_.change -and -not $_.static }
    if ($reset) { Set-DnsClientServerAddress -InterfaceIndex $i -ResetServerAddresses }
    foreach ($f in $a.families) {
      if ($f.static -and ($f.change -or $reset)) { Set-DnsClientServerAddress -InterfaceIndex $i -ServerAddresses @($f.servers) }
    }
    $ik = $ifr + '\' + $a.guid
    foreach ($ip in $all) {
      $sub = if ($ip.Contains(':')) { 'Doh6' } else { 'Doh' }
      $hklm.DeleteSubKeyTree($ik + '\DohInterfaceSettings\' + $sub + '\' + $ip, $false)
    }
    foreach ($r in $a.reg) {
      $k = $hklm.CreateSubKey($ik + '\DohInterfaceSettings\' + $r.sub + '\' + $r.ip)
      if ($r.template) { $k.SetValue('DohTemplate', $r.template, 'String') }
      if ($null -ne $r.flags) { $k.SetValue('DohFlags', [long]$r.flags, 'QWord') }
      $k.Close()
    }
    if (-not $a.hadDoh) { $hklm.DeleteSubKeyTree($ik + '\DohInterfaceSettings', $false) }
    $k = $hklm.OpenSubKey($ik)
    if ($k) { $empty = $k.SubKeyCount -eq 0 -and $k.ValueCount -eq 0; $k.Close(); if ($empty) { $hklm.DeleteSubKey($ik) } }
  } catch { $errors += $_.Exception.Message }
}
foreach ($ip in $all) {
  try {
    $d = @($b.doh) | Where-Object { $_.ip -eq $ip }
    if ($d) { Set-DnsClientDohServerAddress -ServerAddress $ip -DohTemplate $d.template -AllowFallbackToUdp $d.fallback -AutoUpgrade $d.upgrade }
    else { Remove-DnsClientDohServerAddress -ServerAddress $ip -Confirm:$false -ErrorAction SilentlyContinue }
  } catch { $errors += $_.Exception.Message }
}
Clear-DnsClientCache
if ($errors) { throw ($errors -join '; ') }
Remove-Item -Path $file
";

    /// The scripts report a failure as one line on stderr and exit code 1.
    pub fn script(body: &str) -> String {
        format!(
            "{PRELUDE}try {{\n{body}\n}} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}"
        )
    }

    pub fn run(body: &str) -> Result<()> {
        let path = super::backup_path();
        std::fs::create_dir_all(path.parent().context("no state directory")?)?;
        let out = process::command("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script(body)])
            .env("BLKBSTR_DNS_BACKUP", &path)
            .output()
            .context("running powershell")?;
        if !out.status.success() {
            bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(())
    }
}
