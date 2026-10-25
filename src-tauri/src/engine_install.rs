//! Installs the pinned zapret2 release. The download is unprivileged and lands in the user's cache;
//! `blkbstrd install-engine` verifies it and writes it (through `pkexec` on Linux, a UAC prompt on
//! Windows), so the GUI never runs elevated and the user sees an authorisation prompt before
//! anything is written.

use blkbstr_core::{engine_release, paths, process, Platform};
use std::path::Path;

#[cfg(not(windows))]
const DAEMON: &str = "/usr/local/libexec/blkbstrd";

pub fn install() -> Result<(), String> {
    if !matches!(
        Platform::current(),
        Some(Platform::Linux | Platform::Windows)
    ) {
        return Err("installing the engine is only supported on Linux and Windows so far".into());
    }
    let dir = paths::downloads_dir().ok_or("no user cache directory on this platform")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let tarball = dir.join(engine_release::archive_name());

    let result = download(&tarball).and_then(|()| unpack_as_root(&tarball));
    std::fs::remove_file(&tarball).ok();
    result
}

/// curl rather than an HTTP client crate: every desktop Linux has it, Windows 10 ships it, and it
/// keeps TLS out of this binary.
fn download(to: &Path) -> Result<(), String> {
    let out = process::command("curl")
        .args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--proto", "=https", "--max-time", "300", "--output"])
        .arg(to)
        .arg(engine_release::url())
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "curl is not installed, and it is needed to download zapret2".to_owned()
            }
            _ => format!("running curl: {e}"),
        })?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "downloading zapret2: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

#[cfg(windows)]
fn unpack_as_root(archive: &Path) -> Result<(), String> {
    let archive = archive.display().to_string();
    crate::elevate::run(
        &crate::elevate::daemon_path()?,
        &["install-engine", &archive],
        "installing zapret2",
    )
}

#[cfg(not(windows))]
fn unpack_as_root(tarball: &Path) -> Result<(), String> {
    let out = process::command("pkexec")
        .args([DAEMON, "install-engine"])
        .arg(tarball)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "pkexec is not installed (it comes with polkit)".to_owned()
            }
            _ => format!("running pkexec: {e}"),
        })?;
    if out.status.success() {
        return Ok(());
    }
    // pkexec exits 126 when the prompt is dismissed and 127 when it cannot be shown, both silent.
    let said = String::from_utf8_lossy(&out.stderr).trim().to_owned();
    Err(match out.status.code() {
        Some(126) => "the authorisation prompt was dismissed".into(),
        _ if said.is_empty() => format!("the install failed ({})", out.status),
        _ => said,
    })
}
