//! `blkbstrd`: the privileged half of Blockbuster. Runs as a service (root or SYSTEM), owns the
//! engine process and the firewall rules, and exposes a handful of operations over a local socket.

mod engine;
mod firewall;
mod hostlist;
#[cfg(unix)]
mod install;
#[cfg(windows)]
#[path = "install_windows.rs"]
mod install;
#[cfg(windows)]
mod job;
mod supervisor;
#[cfg(any(windows, test))]
mod winservice;
#[cfg(any(windows, test))]
mod zip_install;

use anyhow::{Context, Result};
use blkbstr_core::paths;
use blkbstr_core::protocol::{
    read_message, write_message, EngineStatus, ErrorCode, HostList, Request, Response,
    PROTOCOL_VERSION,
};
use blkbstr_core::registry::{self, Platform};
use engine::Engine;
use interprocess::local_socket::{
    prelude::*, GenericFilePath, GenericNamespaced, ListenerOptions, Stream,
};
use std::io::BufReader;
use std::sync::{Arc, Mutex};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How often the monitor checks on the engine.
const MONITOR_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

type EngineSlot = Mutex<Result<Engine, String>>;

fn main() -> Result<()> {
    let mut argv = std::env::args().skip(1);
    match argv.next().as_deref() {
        Some("install-engine") => {
            let _log_guard = init_logging();
            return install_engine(argv.next());
        }
        #[cfg(windows)]
        Some("install-service") => return winservice::install(&Args::parse(argv)),
        #[cfg(windows)]
        Some("uninstall-service") => return winservice::uninstall(),
        #[cfg(windows)]
        Some("service") => return winservice::run_as_service(Args::parse(argv)),
        _ => {}
    }
    let args = Args::parse(std::env::args().skip(1));
    let _log_guard = init_logging();
    // Unix handles SIGTERM; on Windows a foreground run is for development, and the engine dies
    // with the daemon either way (see `job`).
    #[cfg(unix)]
    return run(&args, spawn_shutdown);
    #[cfg(not(unix))]
    run(&args, |_| Ok(()))
}

/// Binds the socket, brings the engine back and serves requests until the process ends. `ready`
/// gets the engine once the socket is up, which is where each platform hooks its stop signal.
fn run(args: &Args, ready: impl FnOnce(Arc<EngineSlot>) -> Result<()>) -> Result<()> {
    let listener = bind(args)?;
    tracing::info!(version = VERSION, socket = %paths::socket_name(), "blkbstrd listening");

    // One engine, one lock: requests that touch the network stack serialise through it.
    let engine = Arc::new(Mutex::new(open_engine()));

    if let Ok(engine) = engine.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        engine.restore();
    }
    spawn_monitor(Arc::clone(&engine));
    spawn_hostlist_check(Arc::clone(&engine));
    ready(Arc::clone(&engine))?;

    for conn in listener.incoming() {
        match conn {
            Ok(conn) => {
                let engine = Arc::clone(&engine);
                // Thread per connection: clients are one GUI plus a CLI at most.
                std::thread::spawn(move || {
                    if let Err(e) = serve(conn, &engine) {
                        tracing::warn!(error = %e, "connection ended");
                    }
                });
            }
            Err(e) => tracing::warn!(error = %e, "accept failed"),
        }
    }
    Ok(())
}

/// Not the service: a one-shot the GUI runs through `pkexec`, so it has no socket and no engine.
fn install_engine(tarball: Option<String>) -> Result<()> {
    let tarball = tarball.context("usage: blkbstrd install-engine <archive>")?;
    tracing::info!(archive = %tarball, "installing the engine");
    let result = unpack_engine(&tarball);
    match &result {
        Ok(()) => tracing::info!("engine installed"),
        Err(e) => tracing::error!(error = %format!("{e:#}"), "installing the engine failed"),
    }
    // UAC gives the GUI no way to read this process's output, so the reason goes in a file next to
    // the archive for it to pick up.
    #[cfg(windows)]
    if let Err(e) = &result {
        std::fs::write(format!("{tarball}.err"), format!("{e:#}")).ok();
    }
    result
}

#[cfg(any(unix, windows))]
fn unpack_engine(tarball: &str) -> Result<()> {
    install::run(std::path::Path::new(tarball))
}

#[cfg(not(any(unix, windows)))]
fn unpack_engine(tarball: &str) -> Result<()> {
    anyhow::bail!("installing the engine is not supported on this platform yet ({tarball})")
}

/// Takes the engine and rules down before the process exits. The engine is a child outside our
/// process group, so a daemon killed without this leaves NFQUEUE bound and every later start dies
/// with `nfq_create_queue(): Operation not permitted`.
#[cfg(unix)]
fn spawn_shutdown(engine: Arc<EngineSlot>) -> Result<()> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;

    let mut signals =
        Signals::new([SIGTERM, SIGINT, SIGHUP]).context("installing signal handlers")?;
    std::thread::spawn(move || {
        let Some(signal) = signals.forever().next() else {
            return;
        };
        tracing::info!(signal, "shutting down");
        if let Ok(engine) = engine.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            engine.shutdown();
        }
        std::process::exit(0);
    });
    Ok(())
}

/// Restarts an engine that exited by itself. Runs on a timer so the lock is held only briefly.
fn spawn_monitor(engine: Arc<EngineSlot>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(MONITOR_INTERVAL);
        if let Ok(engine) = engine.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            engine.tick();
        }
    });
}

/// Re-checks the learned hosts on a schedule. Its own thread, because a check waits on the
/// network and the engine lock must not be held that long.
fn spawn_hostlist_check(engine: Arc<EngineSlot>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(MONITOR_INTERVAL);
        hostlist::tick(list_in_use(&engine, HostList::Auto));
    });
}

fn list_in_use(engine: &EngineSlot, list: HostList) -> bool {
    engine
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_ok_and(|engine| engine.list_in_use(list))
}

/// A missing engine binary is reported per request, not at startup, so `ping` and `status` still
/// answer and the GUI can say what is wrong.
fn open_engine() -> Result<Engine, String> {
    let Some(platform) = Platform::current() else {
        return Err("this platform has no zapret2 engine".into());
    };
    Engine::new(platform).map_err(|e| {
        tracing::error!(error = %e, "engine unavailable");
        format!("{e:#}")
    })
}

struct Args {
    /// Group id to hand the socket to; without it the socket is root-only and the GUI cannot connect.
    socket_group_gid: Option<u32>,
    /// Windows: SID of the one user who may open the pipe, besides SYSTEM.
    allow_sid: Option<String>,
}

impl Args {
    fn parse(args: impl Iterator<Item = String>) -> Self {
        let mut out = Args {
            socket_group_gid: None,
            allow_sid: None,
        };
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--socket-group-gid" => {
                    out.socket_group_gid = args.next().and_then(|v| v.parse().ok());
                }
                "--allow-sid" => out.allow_sid = args.next(),
                "--version" => {
                    println!("blkbstrd {VERSION}");
                    std::process::exit(0);
                }
                other => eprintln!("blkbstrd: ignoring unknown argument {other}"),
            }
        }
        out
    }
}

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_env("BLKBSTR_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "blkbstrd panicked");
        previous(info);
    }));
    let stderr = fmt::layer().with_writer(std::io::stderr);

    // A daemon that cannot write its log file still has to run; the service manager captures
    // stderr either way.
    let dir = paths::log_dir();
    let file = std::fs::create_dir_all(&dir).ok().map(|()| {
        let appender = tracing_appender::rolling::daily(&dir, "blkbstrd.log");
        tracing_appender::non_blocking(appender)
    });

    match file {
        Some((writer, guard)) => {
            tracing_subscriber::registry()
                .with(filter)
                .with(stderr)
                .with(fmt::layer().with_ansi(false).with_writer(writer))
                .init();
            Some(guard)
        }
        None => {
            tracing_subscriber::registry()
                .with(filter)
                .with(stderr)
                .init();
            tracing::warn!(dir = %dir.display(), "no log directory; logging to stderr only");
            None
        }
    }
}

fn bind(args: &Args) -> Result<interprocess::local_socket::Listener> {
    let name = paths::socket_name();
    #[cfg(not(any(unix, windows)))]
    let _ = args;
    let listener = if paths::socket_is_namespaced() {
        let options = ListenerOptions::new().name(name.clone().to_ns_name::<GenericNamespaced>()?);
        #[cfg(windows)]
        let options = winservice::restrict_pipe(options, args.allow_sid.as_deref())?;
        options.create_sync()
    } else {
        if let Some(parent) = std::path::Path::new(&name).parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        // A leftover socket from a crash makes bind fail with EADDRINUSE; a live one makes
        // connect succeed, in which case another daemon owns it and we must not clobber it.
        if std::path::Path::new(&name).exists()
            && Stream::connect(name.clone().to_fs_name::<GenericFilePath>()?).is_err()
        {
            std::fs::remove_file(&name).ok();
        }
        ListenerOptions::new()
            .name(name.clone().to_fs_name::<GenericFilePath>()?)
            .create_sync()
    }
    .with_context(|| format!("binding {name}"))?;

    #[cfg(unix)]
    set_socket_access(&name, args.socket_group_gid)?;

    Ok(listener)
}

#[cfg(unix)]
fn set_socket_access(path: &str, gid: Option<u32>) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Anyone who can write this socket can reconfigure the firewall, so it is never world-writable.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))
        .with_context(|| format!("chmod 660 {path}"))?;
    match gid {
        Some(gid) => std::os::unix::fs::chown(path, None, Some(gid)).map_err(|e| {
            let why = e.to_string();
            let hint = denial_hint(&why).unwrap_or_default();
            anyhow::anyhow!("chgrp {gid} {path}: {why}{hint}")
        })?,
        None => tracing::warn!(
            "no --socket-group-gid given; socket is root-only and the GUI cannot connect"
        ),
    }
    Ok(())
}

fn serve(conn: Stream, engine: &EngineSlot) -> Result<()> {
    let mut reader = BufReader::new(&conn);
    let mut writer = &conn;
    while let Some(request) = read_message::<Request>(&mut reader)? {
        retry_missing_engine(engine);
        let response = handle(request, engine);
        write_message(&mut writer, &response)?;
    }
    Ok(())
}

/// The engine is installed from the GUI after the daemon is already up, so a failure at startup
/// is looked at again on every request instead of sticking until the next restart.
fn retry_missing_engine(slot: &EngineSlot) {
    let mut slot = slot.lock().unwrap_or_else(|e| e.into_inner());
    if slot.is_ok() {
        return;
    }
    if let Some(mut engine) = Platform::current().and_then(|p| Engine::new(p).ok()) {
        engine.restore();
        *slot = Ok(engine);
    }
}

fn handle(request: Request, engine: &EngineSlot) -> Response {
    // These touch a file and the network, not the engine, so they must not queue behind its lock.
    match &request {
        Request::Hosts { list } => {
            return Response::Hosts(hostlist::status(*list, list_in_use(engine, *list)));
        }
        Request::HostsAdd { list, hosts } => {
            return match hostlist::add_hosts(&hostlist::path_for(*list), hosts) {
                Ok(_) => Response::Ok,
                Err(e) => engine_failed(e),
            };
        }
        Request::HostsRemove { list, hosts } => {
            return match hostlist::remove_hosts(&hostlist::path_for(*list), hosts) {
                Ok(()) => Response::Ok,
                Err(e) => engine_failed(e),
            };
        }
        Request::HostsCheck => {
            hostlist::check_in_background();
            return Response::Ok;
        }
        _ => {}
    }
    // A panic in one connection must not poison the engine for every later one.
    let mut guard = engine.lock().unwrap_or_else(|e| e.into_inner());

    match request {
        Request::Ping { protocol } if protocol != PROTOCOL_VERSION => Response::Error {
            code: ErrorCode::ProtocolMismatch,
            message: format!("daemon speaks protocol v{PROTOCOL_VERSION}, client sent v{protocol}"),
        },
        Request::Ping { .. } => Response::Pong {
            daemon_version: VERSION.into(),
            protocol: PROTOCOL_VERSION,
        },
        Request::Status => match guard.as_ref() {
            Ok(engine) => Response::Status(engine.status()),
            // Without an engine nothing can be running, and the reason belongs in the status
            // rather than in an error the UI would render as a failed request.
            Err(why) => Response::Status(EngineStatus {
                last_error: Some(why.clone()),
                ..EngineStatus::default()
            }),
        },
        Request::Start { config, ephemeral } => {
            if let Err(e) = config.validate() {
                return Response::Error {
                    code: ErrorCode::BadRequest,
                    message: e.to_string(),
                };
            }
            for warning in registry::lint(&config) {
                let strategy = warning.strategy.as_deref().unwrap_or("-");
                tracing::warn!(strategy, "{}", warning.message);
            }
            match guard.as_mut() {
                Ok(engine) => match engine.start(&config, ephemeral) {
                    Ok(()) => Response::Ok,
                    Err(e) => engine_failed(e),
                },
                Err(why) => Response::Error {
                    code: ErrorCode::EngineFailed,
                    message: why.clone(),
                },
            }
        }
        Request::Confirm => match guard.as_mut() {
            Ok(engine) => match engine.confirm() {
                Ok(()) => Response::Ok,
                Err(e) => engine_failed(e),
            },
            Err(why) => Response::Error {
                code: ErrorCode::EngineFailed,
                message: why.clone(),
            },
        },
        Request::Stop => match guard.as_mut() {
            Ok(engine) => match engine.stop() {
                Ok(()) => Response::Ok,
                Err(e) => engine_failed(e),
            },
            Err(why) => Response::Error {
                code: ErrorCode::EngineFailed,
                message: why.clone(),
            },
        },
        Request::Hosts { .. }
        | Request::HostsAdd { .. }
        | Request::HostsRemove { .. }
        | Request::HostsCheck => {
            unreachable!("answered before the engine lock")
        }
    }
}

fn engine_failed(e: anyhow::Error) -> Response {
    tracing::error!(error = %e, "engine operation failed");
    // `{:#}` keeps the anyhow context chain, which is where the actual cause usually is.
    let mut message = format!("{e:#}");
    message.push_str(denial_hint(&message).unwrap_or_default());
    Response::Error {
        code: ErrorCode::EngineFailed,
        message,
    }
}

/// An LSM denial reaches us as a plain "Operation not permitted" from `nft` or the engine, which
/// looks like nonsense for a root process. Names what can actually refuse it.
fn denial_hint(message: &str) -> Option<&'static str> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    ["Operation not permitted", "Permission denied"]
        .iter()
        .any(|s| message.contains(s))
        .then_some(
            " (running as root, so this was refused by AppArmor, SELinux or the capability \
             bounding set rather than by file permissions - `journalctl -k -g 'apparmor|avc'` \
             names the rule; see packaging/linux/README.md)",
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use blkbstr_core::Config;

    /// Covers the paths that must answer sensibly on a machine without zapret2, which is every
    /// machine before onboarding.
    fn no_engine() -> EngineSlot {
        Mutex::new(Err("nfqws2 not found".into()))
    }

    #[test]
    fn a_root_denial_says_what_could_have_refused_it() {
        // Only Linux ships LSM profiles, so only there does the hint point anywhere real.
        let hint = denial_hint("nft rejected the ruleset: Operation not permitted");
        assert_eq!(hint.is_some(), cfg!(target_os = "linux"));
        assert!(denial_hint("nfqws2 not found").is_none());
    }

    #[test]
    fn ping_requires_a_matching_protocol() {
        let engine = no_engine();
        assert!(matches!(
            handle(
                Request::Ping {
                    protocol: PROTOCOL_VERSION
                },
                &engine
            ),
            Response::Pong { .. }
        ));
        assert!(matches!(
            handle(
                Request::Ping {
                    protocol: PROTOCOL_VERSION + 1
                },
                &engine
            ),
            Response::Error {
                code: ErrorCode::ProtocolMismatch,
                ..
            }
        ));
    }

    #[test]
    fn start_rejects_invalid_configs_before_touching_the_engine() {
        let mut config = Config::new("ok");
        config.name = "../escape".into();
        assert!(matches!(
            handle(
                Request::Start {
                    config: Box::new(config),
                    ephemeral: false
                },
                &no_engine()
            ),
            Response::Error {
                code: ErrorCode::BadRequest,
                ..
            }
        ));
    }

    #[test]
    fn status_reports_a_missing_engine_instead_of_failing() {
        let Response::Status(status) = handle(Request::Status, &no_engine()) else {
            panic!("status must always answer with a status");
        };
        assert!(!status.running);
        assert_eq!(status.last_error.as_deref(), Some("nfqws2 not found"));
    }

    #[test]
    fn start_without_an_engine_is_an_engine_failure() {
        assert!(matches!(
            handle(
                Request::Start {
                    config: Box::new(Config::new("ok")),
                    ephemeral: false
                },
                &no_engine()
            ),
            Response::Error {
                code: ErrorCode::EngineFailed,
                ..
            }
        ));
    }

    #[test]
    fn parses_the_socket_group_argument() {
        let args = Args::parse(["--socket-group-gid".to_string(), "1234".to_string()].into_iter());
        assert_eq!(args.socket_group_gid, Some(1234));
        assert_eq!(Args::parse(std::iter::empty()).socket_group_gid, None);
    }
}
