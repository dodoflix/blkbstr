//! Controlling the daemon's service unit from the GUI.
//!
//! Shells out to `systemctl` rather than speaking to systemd over D-Bus. Run unprivileged,
//! `systemctl` already goes through D-Bus and raises a polkit prompt for the operations that need
//! authorisation, which is exactly the behaviour we want and none of the code.
//!
//! It needs a polkit agent in the session. Every mainstream desktop runs one; a bare window
//! manager may not, and there the prompt never appears and the call fails with an authorisation
//! error rather than hanging.

use serde::Serialize;
use std::process::Command;

const UNIT: &str = "blkbstrd.service";

#[derive(Serialize, Default)]
pub struct ServiceStatus {
    /// False when the unit file is not installed at all — the first-run state.
    pub installed: bool,
    /// Starts at boot.
    pub enabled: bool,
    /// Running now.
    pub active: bool,
    /// Set when this platform has no service integration yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported: Option<String>,
}

/// Queries need no authorisation, so these never prompt.
pub fn status() -> ServiceStatus {
    if !cfg!(target_os = "linux") {
        return ServiceStatus {
            unsupported: Some("service control is systemd-only so far".into()),
            ..ServiceStatus::default()
        };
    }

    // `is-enabled` exits non-zero for "disabled", and `is-active` exits 3 for "inactive". Neither
    // is an error, so the exit status is ignored and the word on stdout is what counts.
    let enabled = query(["is-enabled", UNIT]);
    let active = query(["is-active", UNIT]);
    ServiceStatus {
        // "not-found" is systemd's answer for a unit that was never installed.
        installed: enabled.as_deref() != Some("not-found") && enabled.is_some(),
        enabled: matches!(
            enabled.as_deref(),
            Some("enabled") | Some("enabled-runtime")
        ),
        active: active.as_deref() == Some("active"),
        unsupported: None,
    }
}

fn query(args: [&str; 2]) -> Option<String> {
    let out = Command::new("systemctl").args(args).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// Enable or disable starting at boot. Prompts via polkit.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    run(if enabled { "enable" } else { "disable" })
}

/// Start or stop the service now. Prompts via polkit.
pub fn set_active(active: bool) -> Result<(), String> {
    run(if active { "start" } else { "stop" })
}

fn run(verb: &str) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("service control is systemd-only so far".into());
    }
    let out = Command::new("systemctl")
        .args([verb, UNIT])
        .output()
        .map_err(|e| format!("running systemctl: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let detail = stderr.trim();
    Err(if detail.is_empty() {
        format!("systemctl {verb} {UNIT} failed")
    } else {
        detail.to_owned()
    })
}
