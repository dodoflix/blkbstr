//! The zapret2 release Blockbuster installs. Pinned with its hash because the file ends up
//! executing as root: whoever can change what this downloads can run code on the machine.

pub const VERSION: &str = "1.0.5.2";
pub const SHA256: &str = "fb3bcf69e7d86b9fa2d60bd53c956ac06d9dcc3adf9392afd84865f1d94b1158";

/// Where it is unpacked. Matches the upstream layout, which `detect::locate_engine` and
/// `detect::locate_lua_dir` already search.
pub const INSTALL_DIR: &str = "/opt/zapret2";

pub fn url() -> String {
    format!(
        "https://github.com/bol-van/zapret2/releases/download/v{VERSION}/zapret2-v{VERSION}.tar.gz"
    )
}

/// The top-level directory inside the tarball.
pub fn archive_root() -> String {
    format!("zapret2-v{VERSION}")
}

/// The tarball's `binaries/` subdirectory for this CPU, or `None` when upstream ships nothing for it.
pub fn binary_dir() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("linux-x86_64"),
        "aarch64" => Some("linux-arm64"),
        "x86" => Some("linux-x86"),
        "arm" => Some("linux-arm"),
        "riscv64" => Some("linux-riscv64"),
        _ => None,
    }
}
