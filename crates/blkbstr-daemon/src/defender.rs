//! Windows Security flags the WinDivert driver that ships with the engine and removes it as it is
//! written, so the install folder is excluded from scanning before anything lands in it. Only that
//! one folder, only for an install the user asked for and was told about. Best effort: with another
//! antivirus, or Tamper Protection on, the cmdlet fails and the install reports the block instead
//! (see `zip_install`).

use blkbstr_core::process;
use std::path::Path;

fn run(cmdlet: &str, dir: &Path) {
    let dir = dir.display().to_string();
    let script = format!("{cmdlet} -ExclusionPath '{}'", dir.replace('\'', "''"));
    match process::command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
    {
        Ok(out) if out.status.success() => {
            tracing::info!(%dir, cmdlet, "Windows Security exclusion updated");
        }
        Ok(out) => tracing::warn!(
            %dir,
            cmdlet,
            stderr = %String::from_utf8_lossy(&out.stderr).trim(),
            "could not change the Windows Security exclusion"
        ),
        Err(e) => tracing::warn!(error = %e, "could not run powershell"),
    }
}

pub fn exclude(dir: &Path) {
    run("Add-MpPreference", dir);
}

pub fn forget(dir: &Path) {
    run("Remove-MpPreference", dir);
}
