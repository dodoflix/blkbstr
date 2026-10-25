//! Saved configs: plain JSON files in the user's config directory, read and written by the GUI;
//! the daemon only receives a config at start.

use blkbstr_core::{detect, legacy, paths, Config};
use serde::Serialize;
use std::path::PathBuf;

fn dir() -> Result<PathBuf, String> {
    let dir = paths::configs_dir().ok_or("no user config directory on this platform")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Validation happens before the name reaches the filesystem, so `name` can never traverse.
fn path_for(name: &str) -> Result<PathBuf, String> {
    let probe = Config::new(name);
    probe.validate().map_err(|e| e.to_string())?;
    Ok(dir()?.join(format!("{name}.json")))
}

pub fn list() -> Result<Vec<String>, String> {
    let mut names: Vec<String> = std::fs::read_dir(dir()?)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| e.path().file_stem()?.to_str().map(str::to_owned))
        .collect();
    names.sort();
    Ok(names)
}

pub fn load(name: &str) -> Result<Config, String> {
    let path = path_for(name)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let config = Config::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    // A file edited by hand, or arriving by sync, is untrusted input like any other.
    config
        .validate()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<(), String> {
    config.validate().map_err(|e| e.to_string())?;
    let path = path_for(&config.name)?;
    let json = config.to_json().map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn delete(name: &str) -> Result<(), String> {
    let path = path_for(name)?;
    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))
}

#[derive(Serialize)]
pub struct LegacyImport {
    /// The name it was saved under.
    pub name: String,
    /// Where the original `config` was copied, for the user to be told about.
    pub backup: String,
    /// Options that did not map onto a config.
    pub skipped: Vec<String>,
}

/// Imports an existing zapret2 install's `config` as `Legacy` (or `Legacy-2`... if taken). The
/// original is copied aside first and never modified.
pub fn import_legacy() -> Result<LegacyImport, String> {
    let install = detect::existing_install().ok_or("no existing zapret2 install was found")?;
    let source = install.join("config");
    let text =
        std::fs::read_to_string(&source).map_err(|e| format!("{}: {e}", source.display()))?;

    let taken = list()?;
    let name = std::iter::once("Legacy".to_owned())
        .chain((2..).map(|n| format!("Legacy-{n}")))
        .find(|n| !taken.contains(n))
        .unwrap_or_default();
    let imported = legacy::parse(&text, &name)?;

    let backups = paths::backups_dir().ok_or("no user config directory on this platform")?;
    std::fs::create_dir_all(&backups)
        .map_err(|e| format!("creating {}: {e}", backups.display()))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let backup = backups.join(format!("zapret2-config-{stamp}"));
    std::fs::write(&backup, &text).map_err(|e| format!("{}: {e}", backup.display()))?;

    save(&imported.config)?;
    Ok(LegacyImport {
        name,
        backup: backup.display().to_string(),
        skipped: imported.skipped,
    })
}
