//! `blkbstrd` — the privileged half of Blockbuster.
//!
//! Runs as a service (root / SYSTEM), owns the zapret2 engine process and the firewall rules, and
//! exposes exactly four operations over a local socket. The GUI stays unprivileged and can do
//! nothing to the network stack except through this protocol.

mod engine;
mod firewall;
mod supervisor;

use anyhow::{Context, Result};
use blkbstr_core::paths;
use blkbstr_core::protocol::{
    read_message, write_message, EngineStatus, ErrorCode, Request, Response, PROTOCOL_VERSION,
};
use blkbstr_core::registry::{self, Platform};
use engine::Engine;
use interprocess::local_socket::{
    prelude::*, GenericFilePath, GenericNamespaced, ListenerOptions, Stream,
};
use std::io::BufReader;
use std::sync::{Arc, Mutex};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How often the monitor checks on the engine. A crash is noticed within this, which is far below
/// what a user would perceive and far above what would make the poll itself cost anything.
const MONITOR_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

fn main() -> Result<()> {
    let args = Args::parse(std::env::args().skip(1));
    let _log_guard = init_logging();

    let listener = bind(&args)?;
    tracing::info!(version = VERSION, socket = %paths::socket_name(), "blkbstrd listening");

    // One engine, one lock. Every request that touches the network stack serialises through it,
    // which is what stops two clients from applying different configs at the same time.
    let engine = Arc::new(Mutex::new(open_engine()));

    if let Ok(engine) = engine.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        engine.restore();
    }
    spawn_monitor(Arc::clone(&engine));

    for conn in listener.incoming() {
        match conn {
            Ok(conn) => {
                let engine = Arc::clone(&engine);
                // ponytail: thread per connection. Clients are one GUI plus a CLI at most;
                // swap for a poll loop only if that stops being true.
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

/// Watches for an engine that exited by itself, and restarts it. Runs on a timer rather than
/// blocking on the child, so the lock is held only for the instant the check takes.
fn spawn_monitor(engine: Arc<Mutex<Result<Engine, String>>>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(MONITOR_INTERVAL);
        if let Ok(engine) = engine.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            engine.tick();
        }
    });
}

/// A missing engine binary is reported per request rather than at startup: the daemon still has to
/// answer `ping` and `status` so the GUI can say what is wrong instead of showing nothing.
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
    /// Group id to hand the socket to, so an unprivileged GUI in that group can talk to us.
    /// Without it the socket stays root-only and the GUI cannot connect.
    socket_group_gid: Option<u32>,
}

impl Args {
    fn parse(args: impl Iterator<Item = String>) -> Self {
        let mut out = Args {
            socket_group_gid: None,
        };
        let mut args = args.peekable();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--socket-group-gid" => {
                    out.socket_group_gid = args.next().and_then(|v| v.parse().ok());
                }
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
    let listener = if paths::socket_is_namespaced() {
        ListenerOptions::new()
            .name(name.clone().to_ns_name::<GenericNamespaced>()?)
            .create_sync()
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
    #[cfg(not(unix))]
    // ponytail: Windows pipe ACLs unimplemented, so the pipe is default-permissive.
    // Must be restricted to the installing user's group before any Windows release.
    let _ = args;

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

fn serve(conn: Stream, engine: &Mutex<Result<Engine, String>>) -> Result<()> {
    let mut reader = BufReader::new(&conn);
    let mut writer = &conn;
    while let Some(request) = read_message::<Request>(&mut reader)? {
        let response = handle(request, engine);
        write_message(&mut writer, &response)?;
    }
    Ok(())
}

fn handle(request: Request, engine: &Mutex<Result<Engine, String>>) -> Response {
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
/// reads as nonsense in a process already running as root and sends people looking at file modes.
/// Names the things that can actually refuse it.
fn denial_hint(message: &str) -> Option<&'static str> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    ["Operation not permitted", "Permission denied"]
        .iter()
        .any(|s| message.contains(s))
        .then_some(
            " (running as root, so this was refused by AppArmor, SELinux or the capability \
             bounding set rather than by file permissions — `journalctl -k -g 'apparmor|avc'` \
             names the rule; see packaging/linux/README.md)",
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use blkbstr_core::Config;

    /// The engine is deliberately absent: these cover the paths that must answer sensibly on a
    /// machine where zapret2 is not installed, which is every machine before onboarding runs.
    fn no_engine() -> Mutex<Result<Engine, String>> {
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
