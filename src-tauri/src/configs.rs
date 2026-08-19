//! Saved configs. Plain JSON files in the user's config directory — no privileges involved, so
//! the GUI reads and writes them itself and only hands the daemon a config when starting.

use blkbstr_core::{paths, Config};
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
