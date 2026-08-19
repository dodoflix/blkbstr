mod configs;
mod daemon;
mod logs;
mod service;

use blkbstr_core::detect;
use blkbstr_core::protocol::{EngineStatus, Request, Response, PROTOCOL_VERSION};
use blkbstr_core::reachability;
use blkbstr_core::registry::{self, Warning};
use blkbstr_core::Config;
use serde::Serialize;

#[derive(Serialize)]
pub struct DaemonInfo {
    reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    /// Set when the daemon answered but speaks a different protocol, or did not answer at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<String>,
}

/// Never fails: "the daemon is not installed" is the normal first-run state, not an error the UI
/// should render as a crash.
#[tauri::command]
fn daemon_info() -> DaemonInfo {
    match daemon::request(Request::Ping {
        protocol: PROTOCOL_VERSION,
    }) {
        Ok(Response::Pong { daemon_version, .. }) => DaemonInfo {
            reachable: true,
            version: Some(daemon_version),
            problem: None,
        },
        Ok(other) => DaemonInfo {
            reachable: false,
            version: None,
            problem: Some(format!("unexpected reply to ping: {other:?}")),
        },
        Err(e) => DaemonInfo {
            reachable: false,
            version: None,
            problem: Some(e.to_string()),
        },
    }
}

#[tauri::command]
fn engine_status() -> Result<EngineStatus, daemon::Error> {
    match daemon::request(Request::Status)? {
        Response::Status(status) => Ok(status),
        other => Err(daemon::Error::Io(format!(
            "unexpected reply to status: {other:?}"
        ))),
    }
}

#[tauri::command]
fn engine_start(config: Config, ephemeral: bool) -> Result<(), daemon::Error> {
    daemon::request(Request::Start {
        config: Box::new(config),
        ephemeral,
    })
    .map(|_| ())
}

/// Keeps a trial run that would otherwise be undone. The deadline is the daemon's, so this is the
/// only thing that can cancel it.
#[tauri::command]
fn engine_confirm() -> Result<(), daemon::Error> {
    daemon::request(Request::Confirm).map(|_| ())
}

#[tauri::command]
fn engine_stop() -> Result<(), daemon::Error> {
    daemon::request(Request::Stop).map(|_| ())
}

/// Engine, Lua runtime, nftables, distro, and any zapret2 install already on the machine.
/// Unprivileged and local: onboarding has to say what is missing while there is still no daemon
/// to ask, so this never needs one.
#[tauri::command]
fn detect_environment() -> detect::Environment {
    detect::environment()
}

/// Probes each host for a per-site verdict. Local network only — no daemon, nothing uploaded, and
/// the host list never leaves the machine.
///
/// Blocking work on a pool thread rather than the async runtime: an unreachable host costs a full
/// timeout, and the whole point of this call is to sit through those.
#[tauri::command]
async fn check_reachability(hosts: Option<Vec<String>>) -> Result<reachability::Report, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let hosts = hosts.unwrap_or_else(|| {
            reachability::DEFAULT_HOSTS
                .iter()
                .map(|h| (*h).to_owned())
                .collect()
        });
        reachability::check(&hosts, reachability::DEFAULT_TIMEOUT)
    })
    .await
    .map_err(|e| format!("the reachability check did not finish: {e}"))
}

/// What in this config will not do what it looks like it does. Pure and local — the daemon is not
/// consulted, so the UI can lint while the user types.
#[tauri::command]
fn lint_config(config: Config) -> Vec<Warning> {
    registry::lint(&config)
}

/// The Lua functions this build can offer in a strategy editor, newest-known upstream surface.
#[tauri::command]
fn known_functions() -> Vec<&'static str> {
    registry::FUNCTIONS.iter().map(|f| f.name).collect()
}

/// A working config to start from, used by the first-run wizard.
#[tauri::command]
fn starter_config(name: String) -> Config {
    blkbstr_core::render::starter_config(&name)
}

/// Exactly what the daemon will hand the engine. Lets the UI show the real parameter file instead
/// of asking the user to trust that the config means what they think.
#[tauri::command]
fn preview_config(config: Config) -> Result<String, String> {
    let platform =
        blkbstr_core::Platform::current().ok_or("this platform has no zapret2 engine")?;
    Ok(blkbstr_core::render::parameter_file(
        &config,
        &blkbstr_core::render::EngineOptions::new(platform),
    ))
}

#[tauri::command]
fn list_logs() -> Result<Vec<logs::LogFile>, String> {
    logs::list()
}

#[tauri::command]
fn read_log(name: String) -> Result<String, String> {
    logs::tail(&name)
}

/// Writes a diagnostics file and returns its path. Not uploaded anywhere: engine logs name the
/// hosts they saw, so the user reads it before deciding to send it.
#[tauri::command]
fn export_diagnostics(logs_wanted: Vec<String>) -> Result<String, String> {
    let status = match daemon::request(Request::Status) {
        Ok(Response::Status(s)) => {
            serde_json::to_string_pretty(&s).unwrap_or_else(|e| format!("(status: {e})"))
        }
        Ok(other) => format!("(unexpected reply: {other:?})"),
        Err(e) => format!("(daemon unreachable: {e})"),
    };
    logs::export(&status, &logs_wanted)
}

#[tauri::command]
fn list_configs() -> Result<Vec<String>, String> {
    configs::list()
}

#[tauri::command]
fn load_config(name: String) -> Result<Config, String> {
    configs::load(&name)
}

#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    configs::save(&config)
}

/// WebKitGTK's DMA-BUF renderer fails on the NVIDIA proprietary driver under Wayland, killing the
/// process at startup with:
///
/// ```text
/// Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.
/// ```
///
/// Disabling it costs nothing here — this is a settings UI, not a compositor — and a slower
/// renderer beats a window that never opens. Left alone if the user set it themselves.
#[cfg(target_os = "linux")]
fn work_around_webkit_dmabuf() {
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

#[tauri::command]
fn service_status() -> service::ServiceStatus {
    service::status()
}

/// Both prompt via polkit; they fail rather than hang when no polkit agent is running.
#[tauri::command]
fn service_set_enabled(enabled: bool) -> Result<(), String> {
    service::set_enabled(enabled)
}

#[tauri::command]
fn service_set_active(active: bool) -> Result<(), String> {
    service::set_active(active)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    work_around_webkit_dmabuf();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            daemon_info,
            detect_environment,
            check_reachability,
            engine_status,
            engine_start,
            engine_confirm,
            engine_stop,
            lint_config,
            known_functions,
            starter_config,
            preview_config,
            service_status,
            service_set_enabled,
            service_set_active,
            list_logs,
            read_log,
            export_diagnostics,
            list_configs,
            load_config,
            save_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
