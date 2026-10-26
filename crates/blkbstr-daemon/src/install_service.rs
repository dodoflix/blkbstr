//! `blkbstrd install-service`: puts this binary in `/usr/local/libexec`, writes the systemd unit and
//! polkit actions, creates the `blkbstr` group, adds the invoking user to it and (re)starts the
//! service. The GUI runs it through `pkexec` and `packaging/linux/install.sh` through `sudo`, so it
//! is root work the user authorises, not something the service does on a request from the socket.

use anyhow::{bail, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

/// Named by the unit's `ExecStart`, the polkit actions and the AppArmor and SELinux profiles.
pub const LIBEXEC: &str = "/usr/local/libexec/blkbstrd";
const GROUP: &str = "blkbstr";
const UNIT_PATH: &str = "/etc/systemd/system/blkbstrd.service";
const ENV_DIR: &str = "/etc/blkbstr";
const POLKIT_DIR: &str = "/usr/share/polkit-1/actions";

const UNIT: &str = include_str!("../../../packaging/linux/blkbstrd.service");
const POLICIES: [(&str, &str); 2] = [
    (
        "dev.blkbstr.manage-service.policy",
        include_str!("../../../packaging/linux/dev.blkbstr.manage-service.policy"),
    ),
    (
        "dev.blkbstr.install-engine.policy",
        include_str!("../../../packaging/linux/dev.blkbstr.install-engine.policy"),
    ),
];

pub fn run() -> Result<()> {
    if capture("id", &["-u"])? != "0" {
        bail!("installing the helper needs root; the app asks for it through pkexec");
    }
    // The same test as sd_booted(3). A unit file on a machine running another init is dead weight.
    if !Path::new("/run/systemd/system").is_dir() {
        bail!("this installer is systemd-only; see docs/platform-notes.md");
    }

    // The group is the access control: members can drive the engine without further prompts.
    exec("groupadd", &["-f", GROUP])?;
    let gid = group_gid(&capture("getent", &["group", GROUP])?)
        .context("could not read the blkbstr group id")?;
    match invoking_user() {
        Some(user) => {
            exec("usermod", &["-aG", GROUP, "--", &user])?;
            println!("{user} is in the {GROUP} group - log out and back in for it to take effect");
        }
        None => {
            eprintln!("warning: no invoking user found; add yours with: usermod -aG {GROUP} <user>")
        }
    }

    install_self()?;
    write(UNIT_PATH, UNIT, 0o644)?;
    for (name, body) in POLICIES {
        write(&format!("{POLKIT_DIR}/{name}"), body, 0o644)?;
    }
    fs::create_dir_all(ENV_DIR)?;
    write(&format!("{ENV_DIR}/daemon.env"), &env_file(gid), 0o644)?;

    exec("systemctl", &["daemon-reload"])?;
    exec("systemctl", &["enable", "blkbstrd.service"])?;
    // Restart rather than `--now`: on an update the running service must pick up the new binary.
    exec("systemctl", &["restart", "blkbstrd.service"])?;
    println!("the helper is installed and running");
    Ok(())
}

/// Copies the running binary into place unless it already is the installed one. Written beside the
/// target and renamed over it, so a running service keeps its old inode.
fn install_self() -> Result<()> {
    let me = fs::canonicalize(std::env::current_exe()?)?;
    let dest = Path::new(LIBEXEC);
    if fs::canonicalize(dest).is_ok_and(|d| d == me) {
        return Ok(());
    }
    fs::create_dir_all(dest.parent().context("no parent directory")?)?;
    let tmp = dest.with_extension("new");
    fs::copy(&me, &tmp)
        .with_context(|| format!("copying {} to {}", me.display(), tmp.display()))?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    fs::rename(&tmp, dest).with_context(|| format!("installing {LIBEXEC}"))
}

fn write(path: &str, body: &str, mode: u32) -> Result<()> {
    let path = Path::new(path);
    fs::create_dir_all(path.parent().context("no parent directory")?)?;
    fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

/// pkexec names the user who asked in `PKEXEC_UID`, sudo in `SUDO_USER`. Root is nobody to add.
fn invoking_user() -> Option<String> {
    let user = match std::env::var("PKEXEC_UID") {
        Ok(uid) => {
            let uid: u32 = uid.parse().ok()?;
            passwd_name(&capture("getent", &["passwd", &uid.to_string()]).ok()?)?
        }
        Err(_) => std::env::var("SUDO_USER").ok()?,
    };
    (!user.is_empty() && user != "root").then_some(user)
}

fn passwd_name(line: &str) -> Option<String> {
    Some(line.split(':').next()?.to_owned())
}

/// The third field of a `getent group` line, `blkbstr:x:968:alice`.
fn group_gid(line: &str) -> Option<u32> {
    line.split(':').nth(2)?.parse().ok()
}

fn env_file(gid: u32) -> String {
    format!("BLKBSTR_GID={gid}\n")
}

fn exec(program: &str, args: &[&str]) -> Result<()> {
    capture(program, args).map(|_| ())
}

fn capture(program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("running {program}"))?;
    if !out.status.success() {
        bail!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The profiles and the polkit actions pin this path; moving it in one place strands the rest.
    #[test]
    fn the_unit_and_policies_name_the_install_path() {
        assert!(UNIT.contains(&format!("ExecStart={LIBEXEC} ")), "{UNIT}");
        assert!(POLICIES[1].1.contains(LIBEXEC));
    }

    #[test]
    fn reads_the_ids_out_of_getent_lines() {
        assert_eq!(group_gid("blkbstr:x:968:alice"), Some(968));
        assert_eq!(group_gid("blkbstr:x:"), None);
        assert_eq!(
            passwd_name("alice:x:1000:1000::/home/alice:/bin/sh").as_deref(),
            Some("alice")
        );
        assert_eq!(env_file(968), "BLKBSTR_GID=968\n");
    }
}
