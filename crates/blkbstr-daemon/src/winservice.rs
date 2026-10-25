//! Windows service plumbing: registering `blkbstrd` with the service manager in one elevated step,
//! running under it, and fencing off the pipe the GUI talks over. Everything that decides who may
//! reach SYSTEM is a pure function at the top so it is tested on every platform.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

const SYSTEM: &str = "S-1-5-18";
const ADMINISTRATORS: &str = "S-1-5-32-544";
const USERS: &str = "S-1-5-32-545";

/// A SID is spliced into security descriptor strings, so anything but `S-1-<digits>(-<digits>)+`
/// is refused rather than escaped.
fn is_sid(sid: &str) -> bool {
    let Some(rest) = sid.strip_prefix("S-1-") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('-').collect();
    parts.len() >= 2
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn checked_sid(sid: &str) -> Result<&str> {
    if !is_sid(sid) {
        bail!("{sid:?} is not a user SID (expected something like S-1-5-21-1-2-3-1001)");
    }
    Ok(sid)
}

/// Who may open the pipe: SYSTEM, and the one user who installed the service. Not Administrators:
/// access to this pipe is the authority to run the engine, and it is granted to a person.
fn pipe_sddl(sid: &str) -> Result<String> {
    Ok(format!(
        "D:P(A;;GA;;;{SYSTEM})(A;;GRGW;;;{})",
        checked_sid(sid)?
    ))
}

/// Who may control the service. The default descriptor plus start, stop and query for the
/// installing user, so the GUI can start and stop it without a UAC prompt. Deliberately not
/// change-config: that right lets its holder repoint the service at any binary.
fn service_sddl(sid: &str) -> Result<String> {
    Ok(format!(
        "D:(A;;CCLCSWRPWPDTLOCRRC;;;{SYSTEM})\
         (A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;{ADMINISTRATORS})\
         (A;;CCLCSWLOCRRC;;;IU)\
         (A;;LCRPWPLORC;;;{})",
        checked_sid(sid)?
    ))
}

/// The service runs this binary as SYSTEM, so it must sit where only administrators can replace it.
fn is_protected_location(exe: &Path, roots: &[PathBuf]) -> bool {
    let norm = |p: &Path| p.to_string_lossy().to_lowercase().replace('/', "\\");
    let exe = norm(exe);
    roots.iter().any(|root| {
        let root = norm(root);
        let root = root.trim_end_matches('\\');
        !root.is_empty() && exe.starts_with(&format!("{root}\\"))
    })
}

/// `icacls` invocations that make `base` writable by SYSTEM and administrators alone. The state
/// directory decides what SYSTEM runs, and `C:\ProgramData` lets any user create a directory in it
/// before we do, so ownership and every inherited entry are reset, not just the top.
fn lock_down_steps(base: &Path) -> Vec<Vec<String>> {
    let base = base.display().to_string();
    let step = |rest: &[&str]| {
        std::iter::once(base.clone())
            .chain(rest.iter().map(|s| (*s).to_owned()))
            .collect::<Vec<_>>()
    };
    vec![
        step(&["/reset", "/T", "/C", "/Q"]),
        step(&["/setowner", &format!("*{ADMINISTRATORS}"), "/T", "/C", "/Q"]),
        // Users may read: the GUI tails the logs from here.
        step(&[
            "/inheritance:r",
            "/grant:r",
            &format!("*{SYSTEM}:(OI)(CI)F"),
            &format!("*{ADMINISTRATORS}:(OI)(CI)F"),
            &format!("*{USERS}:(OI)(CI)RX"),
            "/Q",
        ]),
    ]
}

#[cfg(windows)]
pub use imp::*;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::Args;

    pub const NAME: &str = "blkbstrd";

    use anyhow::Context;
    use blkbstr_core::{paths, process};
    use interprocess::local_socket::ListenerOptions;
    use interprocess::os::windows::local_socket::ListenerOptionsExt;
    use interprocess::os::windows::security_descriptor::SecurityDescriptor;
    use std::ffi::OsString;
    use std::sync::{mpsc, OnceLock};
    use std::time::{Duration, Instant};
    use windows_service::service::{
        Service, ServiceAccess, ServiceAction, ServiceActionType, ServiceControl,
        ServiceControlAccept, ServiceErrorControl, ServiceExitCode, ServiceFailureActions,
        ServiceFailureResetPeriod, ServiceInfo, ServiceStartType, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{
        self, ServiceControlHandlerResult, ServiceStatusHandle,
    };
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
    use windows_service::{define_windows_service, service_dispatcher};

    /// Fences the pipe to SYSTEM and one user. Without `--allow-sid` the pipe keeps Windows'
    /// default access, which is only acceptable for a foreground run during development.
    pub fn restrict_pipe<'a>(
        options: ListenerOptions<'a>,
        sid: Option<&str>,
    ) -> Result<ListenerOptions<'a>> {
        let Some(sid) = sid else {
            tracing::warn!("no --allow-sid given; the pipe keeps its default access");
            return Ok(options);
        };
        let sddl = widestring::U16CString::from_str(pipe_sddl(sid)?)?;
        let descriptor =
            SecurityDescriptor::deserialize(&sddl).context("building the pipe's access list")?;
        Ok(options.security_descriptor(descriptor))
    }

    // ---- install / uninstall -------------------------------------------------------------

    /// `blkbstrd install-service --allow-sid <SID>`: the one elevated step.
    pub fn install(args: &Args) -> Result<()> {
        let _log = crate::init_logging();
        let result = install_inner(args);
        if let Err(e) = &result {
            tracing::error!(error = %format!("{e:#}"), "installing the service failed");
        }
        result
    }

    fn install_inner(args: &Args) -> Result<()> {
        let sid = checked_sid(
            args.allow_sid
                .as_deref()
                .context("usage: blkbstrd install-service --allow-sid <SID>")?,
        )?;
        let exe = std::env::current_exe().context("finding blkbstrd.exe")?;
        let roots: Vec<PathBuf> = ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"]
            .iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .collect();
        if !is_protected_location(&exe, &roots) {
            bail!(
                "{} is not under Program Files, where only administrators can replace it. A \
                 service started from here would hand SYSTEM to anyone who can write to it",
                exe.display()
            );
        }

        lock_down_data_dir()?;

        let info = ServiceInfo {
            name: NAME.into(),
            display_name: "Blockbuster".into(),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe,
            launch_arguments: ["service", "--allow-sid", sid].map(OsString::from).to_vec(),
            dependencies: vec![],
            account_name: None,
            account_password: None,
        };
        let access = ServiceAccess::CHANGE_CONFIG
            | ServiceAccess::START
            | ServiceAccess::STOP
            | ServiceAccess::QUERY_STATUS;
        let manager = ServiceManager::local_computer(
            None::<&str>,
            ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
        )?;
        // Installing again is how the user moves access to a different account.
        let service = match manager.open_service(NAME, access) {
            Ok(service) => {
                stop_and_wait(&service)?;
                service.change_config(&info)?;
                service
            }
            Err(_) => manager.create_service(&info, access)?,
        };
        service.set_description("Runs the zapret2 engine for Blockbuster.")?;
        service.update_failure_actions(ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86_400)),
            reboot_msg: None,
            command: None,
            actions: Some(vec![
                ServiceAction {
                    action_type: ServiceActionType::Restart,
                    delay: Duration::from_secs(5),
                };
                3
            ]),
        })?;
        set_service_access(&service_sddl(sid)?)?;
        service.start::<OsString>(&[])?;
        tracing::info!(user = sid, "service installed and started");
        println!("Blockbuster's background helper is installed and running.");
        Ok(())
    }

    /// `blkbstrd uninstall-service`: stops and removes the service. Leaves the data directory.
    pub fn uninstall() -> Result<()> {
        let _log = crate::init_logging();
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
        let Ok(service) = manager.open_service(
            NAME,
            ServiceAccess::DELETE | ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
        ) else {
            println!("The service is not installed.");
            return Ok(());
        };
        stop_and_wait(&service)?;
        service.delete()?;
        tracing::info!("service removed");
        // Only an engine Blockbuster installed got an exclusion; one the user put there did not.
        let engine = blkbstr_core::engine_release::install_dir();
        if blkbstr_core::engine_release::installed_in(&engine).is_some() {
            crate::defender::forget(&engine);
        }
        println!("Blockbuster's background helper is removed.");
        Ok(())
    }

    fn stop_and_wait(service: &Service) -> Result<()> {
        if service.query_status()?.current_state == ServiceState::Stopped {
            return Ok(());
        }
        let _ = service.stop();
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if service.query_status()?.current_state == ServiceState::Stopped {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        bail!("the service did not stop within 20 seconds")
    }

    fn set_service_access(sddl: &str) -> Result<()> {
        let out = process::command("sc.exe")
            .args(["sdset", NAME, sddl])
            .output()
            .context("running sc.exe")?;
        if !out.status.success() {
            bail!(
                "sc sdset failed: {}",
                String::from_utf8_lossy(&out.stdout).trim()
            );
        }
        Ok(())
    }

    fn lock_down_data_dir() -> Result<()> {
        let base = paths::state_dir()
            .parent()
            .context("the state directory has no parent")?
            .to_path_buf();
        for dir in [paths::state_dir(), paths::runtime_dir(), paths::log_dir()] {
            std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        for args in lock_down_steps(&base) {
            let out = process::command("icacls.exe")
                .args(&args)
                .output()
                .context("running icacls.exe")?;
            if !out.status.success() {
                bail!(
                    "icacls {} failed: {}",
                    args.join(" "),
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
        }
        Ok(())
    }

    // ---- running under the service manager -------------------------------------------------

    static ARGS: OnceLock<Args> = OnceLock::new();

    define_windows_service!(ffi_service_main, service_main);

    /// `blkbstrd service ...`: what the service manager starts. Blocks until the service stops.
    pub fn run_as_service(args: Args) -> Result<()> {
        let _ = ARGS.set(args);
        service_dispatcher::start(NAME, ffi_service_main)
            .context("connecting to the service manager; this command is started by Windows, not from a terminal")
    }

    fn service_main(_: Vec<OsString>) {
        let args = ARGS.get().expect("set before the dispatcher starts");
        let _log = crate::init_logging();
        if let Err(e) = serve(args) {
            tracing::error!(error = %format!("{e:#}"), "service stopped");
        }
    }

    fn report(handle: ServiceStatusHandle, state: ServiceState, exit: u32) {
        let accepted = if state == ServiceState::Running {
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
        } else {
            ServiceControlAccept::empty()
        };
        let _ = handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accepted,
            exit_code: if exit == 0 {
                ServiceExitCode::Win32(0)
            } else {
                ServiceExitCode::ServiceSpecific(exit)
            },
            checkpoint: 0,
            wait_hint: Duration::from_secs(15),
            process_id: None,
        });
    }

    fn serve(args: &Args) -> Result<()> {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let handle = service_control_handler::register(NAME, move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;
        report(handle, ServiceState::StartPending, 0);

        // Fail closed: a service started without a user to admit would otherwise serve the
        // pipe's default access.
        if args.allow_sid.is_none() {
            report(handle, ServiceState::Stopped, 2);
            bail!("started without --allow-sid; run `blkbstrd install-service` again");
        }

        let result = crate::run(args, |engine| {
            report(handle, ServiceState::Running, 0);
            std::thread::spawn(move || {
                if stop_rx.recv().is_ok() {
                    report(handle, ServiceState::StopPending, 0);
                    tracing::info!("stop requested; shutting down");
                    if let Ok(engine) = engine.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
                        engine.shutdown();
                    }
                    report(handle, ServiceState::Stopped, 0);
                    std::process::exit(0);
                }
            });
            Ok(())
        });
        report(handle, ServiceState::Stopped, 1);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_well_formed_sids_are_accepted() {
        assert!(is_sid("S-1-5-21-1004336348-1177238915-682003330-1001"));
        assert!(is_sid("S-1-5-18"));
        for bad in [
            "",
            "S-1-5",
            "S-1-5-",
            "S-1-5-1)(A;;GA;;;WD",
            "s-1-5-18",
            "S-1-5-a",
            "S-2-5-18",
        ] {
            assert!(!is_sid(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_pipe_admits_system_and_one_user_only() {
        let sddl = pipe_sddl("S-1-5-21-1-2-3-1001").unwrap();
        assert_eq!(sddl, "D:P(A;;GA;;;S-1-5-18)(A;;GRGW;;;S-1-5-21-1-2-3-1001)");
        assert!(pipe_sddl("S-1-5-21-1)(A;;GA;;;WD").is_err());
    }

    #[test]
    fn the_user_may_start_and_stop_the_service_but_not_reconfigure_it() {
        let sddl = service_sddl("S-1-5-21-1-2-3-1001").unwrap();
        let user = sddl.split('(').find(|a| a.contains("1001")).unwrap();
        // CC is query-config, DC is change-config; only the first may appear for anyone but admins.
        assert!(user.contains("RP") && user.contains("WP"), "{user}");
        assert!(
            !user.contains("DC") && !user.contains("WD") && !user.contains("SD"),
            "{user}"
        );
    }

    #[test]
    fn a_service_binary_outside_program_files_is_refused() {
        let roots = [PathBuf::from(r"C:\Program Files")];
        assert!(is_protected_location(
            Path::new(r"c:\program files\Blockbuster\blkbstrd.exe"),
            &roots
        ));
        for outside in [
            r"C:\Users\me\Downloads\blkbstrd.exe",
            r"C:\Program Files Evil\blkbstrd.exe",
            r"C:\Program Files",
        ] {
            assert!(
                !is_protected_location(Path::new(outside), &roots),
                "{outside}"
            );
        }
        assert!(!is_protected_location(Path::new(r"C:\x.exe"), &[]));
    }

    #[test]
    fn the_data_directory_is_reset_before_it_is_locked() {
        let steps = lock_down_steps(Path::new(r"C:\ProgramData\blkbstr"));
        assert_eq!(steps[0][1], "/reset");
        assert!(steps[1].contains(&"/setowner".to_string()));
        let grants = steps[2].join(" ");
        assert!(grants.contains("/inheritance:r"));
        assert!(
            grants.contains("*S-1-5-32-545:(OI)(CI)RX"),
            "users read only: {grants}"
        );
        assert!(!grants.contains("S-1-5-32-545:(OI)(CI)F"));
    }
}
