//! `blkbstrd install-engine <tarball>`: unpacks the pinned zapret2 release into `/opt/zapret2`.
//! The GUI downloads the tarball and runs this through `pkexec`, so the install is root work the
//! user authorises, not something the service does on a request from the socket.

use anyhow::{bail, Context, Result};
use blkbstr_core::engine_release;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

pub fn run(tarball: &Path) -> Result<()> {
    // Read once and hash what was read: the file sits in a user-writable directory, so hashing
    // it and then reopening it would let a swap in between through.
    let bytes = fs::read(tarball).with_context(|| format!("reading {}", tarball.display()))?;
    let dir = engine_release::binary_dir().context("upstream ships no engine for this CPU")?;
    let dest = Path::new(engine_release::INSTALL_DIR);
    unpack(
        &bytes,
        engine_release::SHA256,
        &engine_release::archive_root(),
        dir,
        dest,
    )?;
    engine_release::write_marker(dest)?;
    println!(
        "installed zapret2 {} in {}",
        engine_release::VERSION,
        dest.display()
    );
    Ok(())
}

/// Writes `nfqws2` and `lua/*` under `dest`, and nothing else from the archive: no `config`, so an
/// install made here is never mistaken for one to import from.
fn unpack(bytes: &[u8], sha256: &str, root: &str, binary_dir: &str, dest: &Path) -> Result<()> {
    let got = format!("{:x}", Sha256::digest(bytes));
    if got != sha256 {
        bail!(
            "the download does not match the pinned release (sha256 {got}); nothing was installed"
        );
    }

    let mut files: Vec<(PathBuf, Vec<u8>, u32)> = Vec::new();
    let mut archive = tar::Archive::new(GzDecoder::new(bytes));
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.into_owned();
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let parts: Vec<&str> = rel
            .components()
            .map(|c| match c {
                Component::Normal(s) => s.to_str().unwrap_or(""),
                _ => "",
            })
            .collect();
        let (target, mode) = match parts.as_slice() {
            ["binaries", d, "nfqws2"] if *d == binary_dir => ("nfqws2".into(), 0o755),
            ["lua", name] if !name.is_empty() => (Path::new("lua").join(name), 0o644),
            _ => continue,
        };
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        files.push((target, data, mode));
    }
    for needed in ["nfqws2", "lua/zapret-lib.lua"] {
        if !files.iter().any(|(t, ..)| t == Path::new(needed)) {
            bail!("the release has no {needed} for this CPU");
        }
    }

    for (target, data, mode) in files {
        let path = dest.join(&target);
        let parent = path.parent().context("no parent directory")?;
        fs::create_dir_all(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o755))?;
        // Rename into place so a running engine keeps its old inode and a half-written file is
        // never the one that gets executed.
        let tmp = path.with_extension("blkbstr-new");
        fs::write(&tmp, data).with_context(|| format!("writing {}", tmp.display()))?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(mode))?;
        fs::rename(&tmp, &path).with_context(|| format!("installing {}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (path, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            builder.append_data(&mut header, path, *data).unwrap();
        }
        let tar = builder.into_inner().unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut gz, &tar).unwrap();
        gz.finish().unwrap()
    }

    fn sum(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    #[test]
    fn installs_the_engine_and_lua_and_nothing_else() {
        let bytes = tarball(&[
            ("z-v1/binaries/linux-x86_64/nfqws2", b"engine"),
            ("z-v1/binaries/linux-arm64/nfqws2", b"wrong cpu"),
            ("z-v1/lua/zapret-lib.lua", b"lib"),
            ("z-v1/config.default", b"must not land"),
            ("z-v1/lua/nested/x.lua", b"must not land"),
        ]);
        let dest = std::env::temp_dir().join(format!("blkbstr-install-{}", std::process::id()));
        unpack(&bytes, &sum(&bytes), "z-v1", "linux-x86_64", &dest).unwrap();

        assert_eq!(fs::read(dest.join("nfqws2")).unwrap(), b"engine");
        assert_eq!(fs::read(dest.join("lua/zapret-lib.lua")).unwrap(), b"lib");
        assert_eq!(
            fs::metadata(dest.join("nfqws2"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
        let mut left: Vec<_> = fs::read_dir(&dest)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        left.sort();
        assert_eq!(left, ["lua", "nfqws2"]);
        assert!(!dest.join("lua/nested").exists());
        fs::remove_dir_all(&dest).unwrap();
    }

    #[test]
    fn refuses_a_download_that_is_not_the_pinned_release() {
        let bytes = tarball(&[("z-v1/binaries/linux-x86_64/nfqws2", b"engine")]);
        let dest = std::env::temp_dir().join(format!("blkbstr-refuse-{}", std::process::id()));
        let err = unpack(&bytes, &"0".repeat(64), "z-v1", "linux-x86_64", &dest).unwrap_err();
        assert!(err.to_string().contains("pinned release"), "{err}");
        assert!(!dest.exists());
    }

    #[test]
    fn a_release_missing_the_lua_library_is_not_half_installed() {
        let bytes = tarball(&[("z-v1/binaries/linux-x86_64/nfqws2", b"engine")]);
        let dest = std::env::temp_dir().join(format!("blkbstr-partial-{}", std::process::id()));
        assert!(unpack(&bytes, &sum(&bytes), "z-v1", "linux-x86_64", &dest).is_err());
        assert!(!dest.exists());
    }
}
