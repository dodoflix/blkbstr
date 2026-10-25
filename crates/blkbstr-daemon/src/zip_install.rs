//! Unpacks the engine from upstream's `.zip`, the only archive that carries the Windows binaries.
//! Platform-neutral so the unpacking is tested on every CI leg.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

/// What `winws2.exe` needs beside it: the Cygwin runtime and the WinDivert library and driver.
fn is_engine_file(name: &str) -> bool {
    matches!(name, "winws2.exe" | "cygwin1.dll" | "WinDivert.dll")
        || (name.starts_with("WinDivert") && name.ends_with(".sys"))
}

/// Writes the engine files flat under `dest` and `lua/*` beside them, and nothing else from the
/// archive: not `killall.exe`, not `mdig.exe`, not the shell scripts.
pub fn unpack(bytes: &[u8], sha256: &str, root: &str, binary_dir: &str, dest: &Path) -> Result<()> {
    let got = format!("{:x}", Sha256::digest(bytes));
    if got != sha256 {
        bail!(
            "the download does not match the pinned release (sha256 {got}); nothing was installed"
        );
    }

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("reading the archive")?;
    let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().to_owned();
        let Some(rel) = name.strip_prefix(root).and_then(|r| r.strip_prefix('/')) else {
            continue;
        };
        let parts: Vec<&str> = rel.split('/').collect();
        let target = match parts.as_slice() {
            ["binaries", d, file] if *d == binary_dir && is_engine_file(file) => {
                PathBuf::from(file)
            }
            ["lua", file] if !file.is_empty() => Path::new("lua").join(file),
            _ => continue,
        };
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        files.push((target, data));
    }
    for needed in [
        "winws2.exe",
        "cygwin1.dll",
        "WinDivert.dll",
        "lua/zapret-lib.lua",
    ] {
        if !files.iter().any(|(t, _)| t == Path::new(needed)) {
            bail!("the release has no {needed} for this CPU");
        }
    }
    if !files
        .iter()
        .any(|(t, _)| t.extension().is_some_and(|e| e == "sys"))
    {
        bail!("the release has no WinDivert driver for this CPU");
    }

    for (target, data) in files {
        let path = dest.join(&target);
        fs::create_dir_all(path.parent().context("no parent directory")?)?;
        // Rename into place so a half-written file is never the one that gets executed.
        let tmp = path.with_extension("blkbstr-new");
        fs::write(&tmp, data).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("installing {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (path, data) in entries {
            zip.start_file(*path, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn sum(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn complete() -> Vec<(&'static str, &'static [u8])> {
        vec![
            ("z-v1/binaries/windows-x86_64/winws2.exe", b"engine"),
            ("z-v1/binaries/windows-x86_64/cygwin1.dll", b"cyg"),
            ("z-v1/binaries/windows-x86_64/WinDivert.dll", b"wd"),
            ("z-v1/binaries/windows-x86_64/WinDivert64.sys", b"drv"),
            ("z-v1/lua/zapret-lib.lua", b"lib"),
        ]
    }

    fn scratch(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("blkbstr-zip-{tag}-{}", std::process::id()))
    }

    #[test]
    fn installs_the_engine_and_lua_and_nothing_else() {
        let mut entries = complete();
        entries.extend([
            ("z-v1/binaries/windows-x86/winws2.exe", &b"wrong cpu"[..]),
            ("z-v1/binaries/windows-x86_64/killall.exe", b"must not land"),
            ("z-v1/config.default", b"must not land"),
            ("z-v1/lua/nested/x.lua", b"must not land"),
            ("elsewhere/lua/zapret-lib.lua", b"wrong root"),
        ]);
        let bytes = archive(&entries);
        let dest = scratch("ok");
        unpack(&bytes, &sum(&bytes), "z-v1", "windows-x86_64", &dest).unwrap();

        assert_eq!(fs::read(dest.join("winws2.exe")).unwrap(), b"engine");
        assert_eq!(fs::read(dest.join("lua/zapret-lib.lua")).unwrap(), b"lib");
        let mut left: Vec<_> = fs::read_dir(&dest)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "WinDivert.dll",
                "WinDivert64.sys",
                "cygwin1.dll",
                "lua",
                "winws2.exe"
            ]
        );
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn refuses_a_download_that_is_not_the_pinned_release() {
        let bytes = archive(&complete());
        let dest = scratch("hash");
        let err = unpack(&bytes, &"0".repeat(64), "z-v1", "windows-x86_64", &dest).unwrap_err();
        assert!(err.to_string().contains("pinned release"), "{err}");
        assert!(!dest.exists());
    }

    #[test]
    fn a_release_missing_the_driver_is_not_half_installed() {
        let mut entries = complete();
        entries.retain(|(p, _)| !p.ends_with(".sys"));
        let bytes = archive(&entries);
        let dest = scratch("driver");
        assert!(unpack(&bytes, &sum(&bytes), "z-v1", "windows-x86_64", &dest).is_err());
        assert!(!dest.exists());
    }
}
