//! Filesystem and socket locations shared by the GUI and the daemon.

use directories::ProjectDirs;
use std::path::{Path, PathBuf};

pub const QUALIFIER: &str = "dev";
pub const ORGANIZATION: &str = "blkbstr";
pub const APPLICATION: &str = "blkbstr";

/// Abstract name on Windows (a named pipe), a filesystem socket elsewhere; see [`socket_is_namespaced`].
/// `BLKBSTR_SOCKET` overrides it, which is how the daemon runs unprivileged in development.
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

/// Per-user config root; `None` without a home directory.
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

/// Where the GUI keeps a download until it is installed. Not `/tmp`: a predictable name there is a
/// symlink away from overwriting someone's file.
pub fn downloads_dir() -> Option<PathBuf> {
    dirs().map(|d| d.cache_dir().join("downloads"))
}

/// Daemon state that survives a reboot (the config to bring back on boot). Root-owned, since it
/// decides what runs as root. `BLKBSTR_STATE_DIR` overrides it.
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

/// Hosts the engine has seen fail (`--hostlist-auto`), which it appends to at runtime. Under the
/// state directory so it survives a reboot.
pub fn auto_hostlist() -> PathBuf {
    state_dir().join("autohostlist.txt")
}

/// The user's own list of hosts to unblock (`--hostlist`), which the engine only reads.
pub fn manual_hostlist() -> PathBuf {
    state_dir().join("hostlist.txt")
}

/// Runtime scratch (parameter file, engine pidfile), cleared on reboot. `BLKBSTR_RUNTIME_DIR` overrides it.
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

/// Daemon logs, system-wide; the GUI reads them.
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

/// A path as it goes into a parameter file. winws2 is a Cygwin program, so it reads `C:/dir/file`
/// where a native path with backslashes is not guaranteed to survive its option parser.
pub fn for_engine(path: &Path) -> String {
    let text = path.display().to_string();
    if cfg!(windows) {
        text.replace('\\', "/")
    } else {
        text
    }
}
