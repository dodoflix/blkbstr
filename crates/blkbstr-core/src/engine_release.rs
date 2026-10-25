//! The zapret2 release Blockbuster installs. Pinned with its hash because the file ends up
//! executing as root (or SYSTEM): whoever can change what this downloads can run code on the machine.

use std::path::{Path, PathBuf};

pub const VERSION: &str = "1.0.5.2";
/// The `.tar.gz` that Unix installs from.
pub const SHA256: &str = "fb3bcf69e7d86b9fa2d60bd53c956ac06d9dcc3adf9392afd84865f1d94b1158";
/// The `.zip` that Windows installs from; upstream ships the Windows binaries only in this one.
pub const ZIP_SHA256: &str = "f585590bea6da82ac2c74926f6ac17e6204ee8b53e455c2ba2edae0c58c5d4ac";

/// Where it is unpacked on Unix. Matches the upstream layout, which `detect::locate_engine` and
/// `detect::locate_lua_dir` already search. Use [`install_dir`] to get the right one for this OS.
pub const INSTALL_DIR: &str = "/opt/zapret2";

/// Where the engine is unpacked on this OS. On Windows that is under Program Files, which only
/// administrators can write: the service runs whatever is there as SYSTEM.
pub fn install_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var_os("ProgramFiles")
            .map_or_else(|| PathBuf::from(r"C:\Program Files"), PathBuf::from)
            .join("zapret2")
    } else {
        PathBuf::from(INSTALL_DIR)
    }
}

/// The archive this OS installs from, and the hash it must have.
pub fn archive_name() -> String {
    let ext = if cfg!(windows) { "zip" } else { "tar.gz" };
    format!("zapret2-v{VERSION}.{ext}")
}

pub fn sha256() -> &'static str {
    if cfg!(windows) {
        ZIP_SHA256
    } else {
        SHA256
    }
}

pub fn url() -> String {
    format!(
        "https://github.com/bol-van/zapret2/releases/download/v{VERSION}/{}",
        archive_name()
    )
}

/// The top-level directory inside the archive.
pub fn archive_root() -> String {
    format!("zapret2-v{VERSION}")
}

/// The archive's `binaries/` subdirectory for this CPU, or `None` when upstream ships nothing for it.
/// Windows on ARM is `None` on purpose: upstream's answer is the x64 build with a test-signed
/// driver, which is not something to install behind a button.
pub fn binary_dir() -> Option<&'static str> {
    match (cfg!(windows), std::env::consts::ARCH) {
        (true, "x86_64") => Some("windows-x86_64"),
        (true, "x86") => Some("windows-x86"),
        (false, "x86_64") => Some("linux-x86_64"),
        (false, "aarch64") => Some("linux-arm64"),
        (false, "x86") => Some("linux-x86"),
        (false, "arm") => Some("linux-arm"),
        (false, "riscv64") => Some("linux-riscv64"),
        _ => None,
    }
}

const MARKER: &str = ".blkbstr-release";

/// Written by the installer after a successful unpack. Without it there is no telling which
/// upstream release a `nfqws2` came from, and an install Blockbuster did not make is not ours to
/// replace.
pub fn write_marker(dir: &Path) -> std::io::Result<()> {
    std::fs::write(dir.join(MARKER), VERSION)
}

/// The release Blockbuster installed into `dir`, or `None` for an install it did not make.
pub fn installed_in(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(MARKER)).ok()?;
    Some(text.trim().to_owned()).filter(|v| !v.is_empty())
}

/// The pinned release, when the install in `dir` is Blockbuster's and older or newer than it.
pub fn update_for(dir: &Path) -> Option<&'static str> {
    installed_in(dir).filter(|v| v != VERSION).map(|_| VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_our_own_stale_install_is_offered_an_update() {
        let dir = std::env::temp_dir().join(format!("blkbstr-marker-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(update_for(&dir), None, "no marker: not ours");

        write_marker(&dir).unwrap();
        assert_eq!(installed_in(&dir).as_deref(), Some(VERSION));
        assert_eq!(update_for(&dir), None, "current");

        std::fs::write(dir.join(MARKER), "0.0.1\n").unwrap();
        assert_eq!(update_for(&dir), Some(VERSION));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
