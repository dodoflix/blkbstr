//! Reading the daemon's and engine's logs. Unprivileged: the daemon writes files under `log_dir()`
//! and the GUI tails the last bytes on a timer.

use blkbstr_core::paths;
use serde::Serialize;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

/// How much of a file the viewer reads: a crash and its lead-up, without a 200 MB string.
const TAIL_BYTES: u64 = 256 * 1024;

#[derive(Serialize)]
pub struct LogFile {
    pub name: String,
    pub size: u64,
    /// Unix epoch seconds, so the UI can sort and show "most recent first".
    pub modified: Option<u64>,
}

/// Rejects anything but a plain filename inside the log directory; the name comes from the UI.
fn path_for(name: &str) -> Result<PathBuf, String> {
    let bad = name.is_empty()
        || name.contains(['/', '\\', '\0'])
        || name.starts_with('.')
        || name == ".."
        || PathBuf::from(name).components().count() != 1;
    if bad {
        return Err(format!("{name:?} is not a log file name"));
    }
    // The daemon's directory first; the app's own log lives in the per-user one.
    let daemon = paths::log_dir().join(name);
    Ok(match paths::app_log_dir().map(|d| d.join(name)) {
        Some(app) if !daemon.exists() && app.exists() => app,
        _ => daemon,
    })
}

fn files_in(dir: &std::path::Path) -> Result<Vec<std::fs::DirEntry>, String> {
    match std::fs::read_dir(dir) {
        Ok(entries) => Ok(entries.filter_map(Result::ok).collect()),
        // No directory means nothing has run yet, which is not an error worth a red banner.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("{}: {e}", dir.display())),
    }
}

pub fn list() -> Result<Vec<LogFile>, String> {
    let mut entries = files_in(&paths::log_dir())?;
    if let Some(app) = paths::app_log_dir() {
        entries.extend(files_in(&app).unwrap_or_default());
    }

    let mut files: Vec<LogFile> = entries
        .into_iter()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            Some(LogFile {
                name: e.file_name().to_str()?.to_owned(),
                size: meta.len(),
                modified: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()),
            })
        })
        .collect();
    files.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
    Ok(files)
}

/// The last [`TAIL_BYTES`] of a log, minus any partial first line.
pub fn tail(name: &str) -> Result<String, String> {
    let path = path_for(name)?;
    let mut file = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();

    let truncated = len > TAIL_BYTES;
    if truncated {
        file.seek(SeekFrom::Start(len - TAIL_BYTES))
            .map_err(|e| e.to_string())?;
    }
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;

    // Logs are UTF-8, but a tail can start mid-character; lossy beats refusing to show anything.
    let mut text = String::from_utf8_lossy(&buf).into_owned();
    if truncated {
        // Seeking to a byte offset almost never lands on a line start.
        if let Some(nl) = text.find('\n') {
            text = text[nl + 1..].to_owned();
        }
    }
    Ok(text)
}

/// Writes one file with everything a bug report needs and returns its path. Not uploaded: engine
/// logs name the hosts they saw, so the user reads it first.
pub fn export(status: &str, logs: &[String]) -> Result<String, String> {
    let dir = paths::config_dir().ok_or("no user config directory on this platform")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("blkbstr-diagnostics-{stamp}.txt"));

    let mut out = String::new();
    out.push_str("# Blockbuster diagnostics\n\n");
    out.push_str(&format!("app version: {}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!("platform: {}\n\n", std::env::consts::OS));
    out.push_str("## Status\n\n");
    out.push_str(status);
    out.push_str("\n\n");

    for name in logs {
        out.push_str(&format!("## {name}\n\n"));
        match tail(name) {
            Ok(text) => out.push_str(&text),
            Err(e) => out.push_str(&format!("(could not read: {e})\n")),
        }
        out.push_str("\n\n");
    }

    std::fs::write(&path, out).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_names_that_are_not_plain_files() {
        for name in [
            "",
            "..",
            "../../etc/passwd",
            "a/b",
            ".hidden",
            "/etc/passwd",
        ] {
            assert!(path_for(name).is_err(), "{name:?} should be rejected");
        }
        assert!(path_for("blkbstrd.log.2026-08-17").is_ok());
    }

    // Sets BLKBSTR_LOG_DIR, which is process-global: any further test that reads `log_dir()`
    // will see this directory when they run in parallel.
    #[test]
    fn a_tailed_file_never_starts_mid_line() {
        let dir = std::env::temp_dir().join(format!("blkbstr-tail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("BLKBSTR_LOG_DIR", &dir);

        // Longer than TAIL_BYTES, so the read starts partway through a line.
        let line = "the quick brown fox jumps over the lazy dog\n";
        let content = line.repeat((TAIL_BYTES as usize / line.len()) + 100);
        std::fs::write(dir.join("big.log"), &content).unwrap();

        let tailed = tail("big.log").unwrap();
        assert!(tailed.len() < content.len(), "should have been truncated");
        assert!(
            tailed.starts_with(line),
            "tail begins mid-line: {:?}",
            &tailed[..40.min(tailed.len())]
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
