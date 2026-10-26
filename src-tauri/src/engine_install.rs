//! Installs the pinned zapret2 release. On Linux the download is unprivileged and lands in the
//! user's cache; `blkbstrd install-engine` verifies it and writes it through `pkexec`. On Windows
//! `blkbstrd install-engine` does both behind a UAC prompt, because Defender blocks any copy of the
//! archive left on disk. Either way the GUI never runs elevated and the user sees an authorisation
//! prompt before anything is written.

#[cfg(not(windows))]
use blkbstr_core::process;
use blkbstr_core::{engine_release, paths, Platform};
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

    let result = fetch_and_install(&tarball);
    std::fs::remove_file(&tarball).ok();
    match &result {
        Ok(()) => tracing::info!("zapret2 installed"),
        Err(e) => tracing::error!(error = %e, "installing zapret2 failed"),
    }
    result
}

#[cfg(not(windows))]
fn fetch_and_install(tarball: &Path) -> Result<(), String> {
    tracing::info!(url = %engine_release::url(), "downloading zapret2");
    download(tarball)?;
    let size = std::fs::metadata(tarball).map_or(0, |m| m.len());
    tracing::info!(
        size,
        "downloaded zapret2; asking for permission to install it"
    );
    unpack_as_root(tarball)
}

/// The elevated step downloads; the path only anchors the file it reports a failure in.
#[cfg(windows)]
fn fetch_and_install(anchor: &Path) -> Result<(), String> {
    tracing::info!("asking for permission to download and install zapret2");
    unpack_as_root(anchor)
}

/// curl rather than an HTTP client crate: every desktop Linux has it, Windows 10 ships it, and it
/// keeps TLS out of this binary.
#[cfg(not(windows))]
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
    // `blkbstrd install-engine` leaves its failure here: the elevated process has no output we can read.
    let reason_file = format!("{}.err", archive.display());
    std::fs::remove_file(&reason_file).ok();
    let archive = archive.display().to_string();
    let result = crate::elevate::run(
        &crate::elevate::daemon_path()?,
        &["install-engine", &archive],
        "installing zapret2",
    );
    let reason = std::fs::read_to_string(&reason_file).ok();
    std::fs::remove_file(&reason_file).ok();
    result.map_err(|e| reason.filter(|r| !r.is_empty()).unwrap_or(e))
}

#[cfg(not(windows))]
fn unpack_as_root(tarball: &Path) -> Result<(), String> {
    crate::pkexec::run(
        Path::new(DAEMON),
        &["install-engine", &tarball.to_string_lossy()],
        "the install",
    )
}
