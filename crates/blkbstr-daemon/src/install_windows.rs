//! `blkbstrd install-engine`: downloads the pinned zapret2 release and unpacks it into Program Files.
//! The GUI runs this elevated through UAC, so the install is administrator work the user authorises,
//! not something the service does on a request from the pipe.
//!
//! The archive is fetched into memory here rather than by the GUI into the user's cache: it carries
//! the WinDivert driver, so Defender blocks any read of a copy on disk (`os error 225`), and an
//! exclusion added afterwards does not lift that.

use crate::zip_install;
use anyhow::{bail, Context, Result};
use blkbstr_core::{engine_release, process};

/// curl ships with Windows 10 and later, which keeps TLS out of this binary.
fn download() -> Result<Vec<u8>> {
    let out = process::command("curl.exe")
        .args(["--fail", "--silent", "--show-error", "--location"])
        .args(["--proto", "=https", "--max-time", "300", "--output", "-"])
        .arg(engine_release::url())
        .output()
        .context("running curl")?;
    if !out.status.success() {
        bail!(
            "downloading zapret2: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(out.stdout)
}

pub fn run() -> Result<()> {
    let dir = engine_release::binary_dir().context("upstream ships no engine for this CPU")?;
    let dest = engine_release::install_dir();
    crate::defender::exclude(&dest);
    let bytes = download()?;
    zip_install::unpack(
        &bytes,
        engine_release::sha256(),
        &engine_release::archive_root(),
        dir,
        &dest,
    )?;
    engine_release::write_marker(&dest)?;
    println!(
        "installed zapret2 {} in {}",
        engine_release::VERSION,
        dest.display()
    );
    Ok(())
}
