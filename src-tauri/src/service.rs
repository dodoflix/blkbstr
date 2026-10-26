//! Controls the daemon's service. Linux shells out to `systemctl`, which raises a polkit prompt for
//! operations that need authorisation (without a polkit agent the call fails instead of hanging).
//! Windows uses `sc.exe`: starting and stopping needs no prompt, because the install step granted
//! that right to the installing user; changing the start type and installing go through UAC.

use serde::Serialize;
use std::process::Command;

const UNIT: &str = "blkbstrd.service";

#[derive(Serialize, Default)]
pub struct ServiceStatus {
    /// False when the unit file is not installed at all - the first-run state.
    pub installed: bool,
    /// Starts at boot.
    pub enabled: bool,
    /// Running now.
    pub active: bool,
    /// Set when this platform has no service integration yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsupported: Option<String>,
}

/// Queries need no authorisation, so these never prompt.
#[cfg_attr(windows, allow(unreachable_code))]
pub fn status() -> ServiceStatus {
    #[cfg(windows)]
    return windows::status();
    if !cfg!(target_os = "linux") {
        return ServiceStatus {
            unsupported: Some("service control is systemd-only so far".into()),
            ..ServiceStatus::default()
        };
    }

    // `is-enabled` exits non-zero for "disabled", and `is-active` exits 3 for "inactive". Neither
    // is an error, so the exit status is ignored and the word on stdout is what counts.
    let enabled = query(["is-enabled", UNIT]);
    let active = query(["is-active", UNIT]);
    ServiceStatus {
        // "not-found" is systemd's answer for a unit that was never installed.
        installed: enabled.as_deref() != Some("not-found") && enabled.is_some(),
        enabled: matches!(
            enabled.as_deref(),
            Some("enabled") | Some("enabled-runtime")
        ),
        active: active.as_deref() == Some("active"),
        unsupported: None,
    }
}

fn query(args: [&str; 2]) -> Option<String> {
    let out = Command::new("systemctl").args(args).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// Enable or disable starting at boot. Prompts via polkit.
#[cfg_attr(windows, allow(unreachable_code))]
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    return windows::set_enabled(enabled);
    run(if enabled { "enable" } else { "disable" })
}

/// Start or stop the service now. Prompts via polkit.
#[cfg_attr(windows, allow(unreachable_code))]
pub fn set_active(active: bool) -> Result<(), String> {
    #[cfg(windows)]
    return windows::set_active(active);
    run(if active { "start" } else { "stop" })
}

/// Registers the service, which takes administrator rights once.
pub fn install() -> Result<(), String> {
    #[cfg(windows)]
    return windows::install();
    #[cfg(not(windows))]
    return crate::pkexec::run(
        &packaged_daemon()?,
        &["install-service"],
        "installing the helper",
    );
}

/// Where the deb and rpm put the helper. `install-service` copies it to its permanent home, so the
/// service never runs from here.
#[cfg(not(windows))]
const PACKAGED_DAEMON: &str = "/usr/lib/blockbuster/blkbstrd";

/// The packaged helper, or the one built next to the app in a source checkout. An AppImage has
/// neither: root cannot read its FUSE mount, so there is nothing safe to run as root.
#[cfg(not(windows))]
fn packaged_daemon() -> Result<std::path::PathBuf, String> {
    let packaged = std::path::PathBuf::from(PACKAGED_DAEMON);
    let sibling = std::env::current_exe()
        .map_err(|e| format!("finding the app: {e}"))?
        .with_file_name("blkbstrd");
    [packaged, sibling].into_iter().find(|p| p.is_file()).ok_or_else(|| {
        "the helper is not installed with this copy of Blockbuster. Install the deb, rpm or AUR \
         package, or run packaging/linux/install.sh from a source checkout"
            .into()
    })
}

#[cfg_attr(windows, allow(dead_code))]
fn run(verb: &str) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("service control is systemd-only so far".into());
    }
    let out = Command::new("systemctl")
        .args([verb, UNIT])
        .output()
        .map_err(|e| format!("running systemctl: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let detail = stderr.trim();
    Err(if detail.is_empty() {
        format!("systemctl {verb} {UNIT} failed")
    } else {
        detail.to_owned()
    })
}

/// `sc.exe` prints English keys in every locale, so the numeric codes are what is parsed.
#[cfg(any(windows, test))]
mod windows {
    use super::ServiceStatus;

    #[cfg(windows)]
    const UNIT_NAME: &str = "blkbstrd";

    /// The first number after `key :` in `sc` output, e.g. `STATE : 4  RUNNING` gives 4.
    fn code(text: &str, key: &str) -> Option<u32> {
        text.lines()
            .map(str::trim)
            .find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix(':'))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    }

    fn parse(query: &str, config: &str) -> ServiceStatus {
        ServiceStatus {
            installed: true,
            // 2 is AUTO_START.
            enabled: code(config, "START_TYPE") == Some(2),
            // 4 is RUNNING.
            active: code(query, "STATE") == Some(4),
            unsupported: None,
        }
    }

    /// The SID from `whoami /user /fo csv /nh`, e.g. `"PC\\me","S-1-5-21-1-2-3-1001"`.
    fn parse_sid(csv: &str) -> Option<String> {
        let sid = csv.trim().rsplit(',').next()?.trim().trim_matches('"');
        let ok = sid.starts_with("S-1-")
            && sid
                .bytes()
                .all(|b| b == b'S' || b == b'-' || b.is_ascii_digit());
        ok.then(|| sid.to_owned())
    }

    #[cfg(windows)]
    fn sc(args: &[&str]) -> Option<(bool, String)> {
        let out = blkbstr_core::process::command("sc.exe")
            .args(args)
            .output()
            .ok()?;
        Some((
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        ))
    }

    #[cfg(windows)]
    pub fn status() -> ServiceStatus {
        // 1060 is "service does not exist", the first-run state, and `sc` exits non-zero for it.
        match (sc(&["query", UNIT_NAME]), sc(&["qc", UNIT_NAME])) {
            (Some((true, query)), Some((true, config))) => parse(&query, &config),
            _ => ServiceStatus::default(),
        }
    }

    #[cfg(windows)]
    pub fn set_active(active: bool) -> Result<(), String> {
        let verb = if active { "start" } else { "stop" };
        match sc(&[verb, UNIT_NAME]) {
            Some((true, _)) => Ok(()),
            Some((false, said)) => Err(said.trim().to_owned()),
            None => Err("running sc.exe failed".into()),
        }
    }

    #[cfg(windows)]
    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        let start = if enabled { "auto" } else { "demand" };
        crate::elevate::run(
            std::path::Path::new("sc.exe"),
            &["config", UNIT_NAME, "start=", start],
            "changing when the helper starts",
        )
    }

    #[cfg(windows)]
    pub fn install() -> Result<(), String> {
        let who = blkbstr_core::process::command("whoami.exe")
            .args(["/user", "/fo", "csv", "/nh"])
            .output()
            .map_err(|e| format!("running whoami: {e}"))?;
        let sid = parse_sid(&String::from_utf8_lossy(&who.stdout))
            .ok_or("could not work out which Windows account is running Blockbuster")?;
        let daemon = crate::elevate::daemon_path()?;
        crate::elevate::run(
            &daemon,
            &["install-service", "--allow-sid", &sid],
            "installing the helper",
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reads_a_running_automatic_service() {
            let query = "SERVICE_NAME: blkbstrd \n        TYPE               : 10  WIN32_OWN_PROCESS  \n        STATE              : 4  RUNNING \n                                (STOPPABLE, NOT_PAUSABLE, ACCEPTS_SHUTDOWN)\n        WIN32_EXIT_CODE    : 0  (0x0)\n";
            let config = "        TYPE               : 10  WIN32_OWN_PROCESS\n        START_TYPE         : 2   AUTO_START\n";
            let s = parse(query, config);
            assert!(s.installed && s.enabled && s.active);
            let stopped = query.replace("4  RUNNING", "1  STOPPED");
            let manual = config.replace("2   AUTO_START", "3   DEMAND_START");
            let s = parse(&stopped, &manual);
            assert!(!s.enabled && !s.active);
        }

        #[test]
        fn takes_the_sid_and_nothing_else_from_whoami() {
            assert_eq!(
                parse_sid("\"PC\\me\",\"S-1-5-21-1-2-3-1001\"\r\n").as_deref(),
                Some("S-1-5-21-1-2-3-1001")
            );
            assert_eq!(parse_sid("\"PC\\me\",\"S-1-5-1\"; calc\""), None);
            assert_eq!(parse_sid("INFO: nothing"), None);
        }
    }
}
