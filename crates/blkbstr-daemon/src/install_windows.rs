//! `blkbstrd install-engine <zip>`: unpacks the pinned zapret2 release into Program Files. The GUI
//! downloads the archive and runs this elevated through UAC, so the install is administrator work
//! the user authorises, not something the service does on a request from the pipe.

use crate::zip_install;
use anyhow::{Context, Result};
use blkbstr_core::engine_release;
use std::path::Path;

pub fn run(archive: &Path) -> Result<()> {
    // Read once and hash what was read: the file sits in a user-writable directory, so hashing
    // it and then reopening it would let a swap in between through.
    let bytes = std::fs::read(archive).with_context(|| format!("reading {}", archive.display()))?;
    let dir = engine_release::binary_dir().context("upstream ships no engine for this CPU")?;
    let dest = engine_release::install_dir();
    zip_install::unpack(
        &bytes,
        engine_release::sha256(),
        &engine_release::archive_root(),
        dir,
        &dest,
    )?;
    println!(
        "installed zapret2 {} in {}",
        engine_release::VERSION,
        dest.display()
    );
    Ok(())
}
