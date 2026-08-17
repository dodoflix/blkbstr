//! Filesystem and socket locations. Lives in `core` because the GUI and the daemon have to agree
//! on every one of them.

use directories::ProjectDirs;
use std::path::PathBuf;

pub const QUALIFIER: &str = "dev";
pub const ORGANIZATION: &str = "blkbstr";
pub const APPLICATION: &str = "blkbstr";

/// Abstract name on Windows (a named pipe), a filesystem socket elsewhere. Pass to
/// `interprocess`'s `to_ns_name` / `to_fs_name` respectively; [`socket_is_namespaced`] says which.
///
/// `BLKBSTR_SOCKET` overrides it, which is how the daemon runs unprivileged during development —
/// the default lives under `/run` and needs root to create.
pub fn socket_name() -> String {
    if let Ok(custom) = std::env::var("BLKBSTR_SOCKET") {
        return custom;
    }
    if socket_is_namespaced() {
        "blkbstrd.sock".into()
    } else {
        "/run/blkbstr/blkbstrd.sock".into()
    }
}

pub const fn socket_is_namespaced() -> bool {
    cfg!(windows)
}

fn dirs() -> Option<ProjectDirs> {
    ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
}

/// Per-user config root. `None` only when the platform has no home directory to speak of.
pub fn config_dir() -> Option<PathBuf> {
    dirs().map(|d| d.config_dir().to_path_buf())
}

/// Saved named configs, one JSON file per config.
pub fn configs_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("configs"))
}

/// Backups taken before importing or overwriting an existing zapret2 installation's config.
pub fn backups_dir() -> Option<PathBuf> {
    config_dir().map(|d| d.join("backups"))
}

/// Daemon state that has to outlive a reboot — currently the config to bring back up on boot.
/// Root-owned: it decides what runs as root, so it is not writable by the GUI's user.
///
/// `BLKBSTR_STATE_DIR` overrides it, for running the daemon unprivileged during development.
pub fn state_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("BLKBSTR_STATE_DIR") {
        return PathBuf::from(custom);
    }
    if cfg!(windows) {
        PathBuf::from(r"C:\ProgramData\blkbstr\state")
    } else {
        PathBuf::from("/var/lib/blkbstr")
    }
}

/// Runtime scratch — the rendered parameter file and the engine pidfile. Cleared by the OS on
/// reboot, unlike [`state_dir`]. `BLKBSTR_RUNTIME_DIR` overrides it for development.
pub fn runtime_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("BLKBSTR_RUNTIME_DIR") {
        return PathBuf::from(custom);
    }
    if cfg!(windows) {
        PathBuf::from(r"C:\ProgramData\blkbstr\run")
    } else {
        PathBuf::from("/run/blkbstr")
    }
}

/// Where the daemon writes rotating logs. System-wide: the daemon is a service, not a user
/// process, and the GUI tails these files read-only.
pub fn log_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("BLKBSTR_LOG_DIR") {
        return PathBuf::from(custom);
    }
    if cfg!(windows) {
        PathBuf::from(r"C:\ProgramData\blkbstr\logs")
    } else {
        PathBuf::from("/var/log/blkbstr")
    }
}
