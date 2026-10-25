//! Installs the pinned zapret2 release. The download is unprivileged and lands in the user's cache;
//! `pkexec blkbstrd install-engine` verifies it and writes it, so the GUI never runs as root and
//! the user sees an authorisation prompt before anything is written.

use blkbstr_core::{engine_release, paths, Platform};
use std::path::Path;
use std::process::Command;

const DAEMON: &str = "/usr/local/libexec/blkbstrd";

pub fn install() -> Result<(), String> {
    if Platform::current() != Some(Platform::Linux) {
        return Err("installing the engine is only supported on Linux so far".into());
    }
    let dir = paths::downloads_dir().ok_or("no user cache directory on this platform")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    let tarball = dir.join(format!("zapret2-v{}.tar.gz", engine_release::VERSION));

    let result = download(&tarball).and_then(|()| unpack_as_root(&tarball));
    std::fs::remove_file(&tarball).ok();
    result
}

/// curl rather than an HTTP client crate: every desktop Linux has it, and it keeps TLS out of this binary.
fn download(to: &Path) -> Result<(), String> {
    let out = Command::new("curl")
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

fn unpack_as_root(tarball: &Path) -> Result<(), String> {
    let out = Command::new("pkexec")
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
