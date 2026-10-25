mod applog;
mod configs;
mod daemon;
#[cfg(any(windows, test))]
mod elevate;
mod engine_install;
mod logs;
mod service;
mod tray;
mod update_watch;
mod updater;

use blkbstr_core::candidates;
use blkbstr_core::detect;
use blkbstr_core::protocol::{
    EngineStatus, HostList, HostsStatus, Request, Response, PROTOCOL_VERSION,
};
use blkbstr_core::reachability;
use blkbstr_core::registry::{self, Warning};
use blkbstr_core::Config;
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
pub struct DaemonInfo {
    reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    /// Set when the daemon answered but speaks a different protocol, or did not answer at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    problem: Option<String>,
}

/// Never fails: "the daemon is not installed" is the normal first-run state.
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

/// Keeps a trial run that would otherwise be undone; the deadline is the daemon's.
#[tauri::command]
fn engine_confirm() -> Result<(), daemon::Error> {
    daemon::request(Request::Confirm).map(|_| ())
}

#[tauri::command]
fn engine_stop() -> Result<(), daemon::Error> {
    daemon::request(Request::Stop).map(|_| ())
}

/// The learned or the user's own host list, with the learned list's re-check state.
#[tauri::command]
fn hosts(list: HostList) -> Result<HostsStatus, daemon::Error> {
    match daemon::request(Request::Hosts { list })? {
        Response::Hosts(status) => Ok(status),
        other => Err(daemon::Error::Io(format!(
            "unexpected reply to hosts: {other:?}"
        ))),
    }
}

#[tauri::command]
fn hosts_add(list: HostList, hosts: Vec<String>) -> Result<(), daemon::Error> {
    daemon::request(Request::HostsAdd { list, hosts }).map(|_| ())
}

#[tauri::command]
fn hosts_remove(list: HostList, hosts: Vec<String>) -> Result<(), daemon::Error> {
    daemon::request(Request::HostsRemove { list, hosts }).map(|_| ())
}

/// Returns at once; the check runs in the daemon and shows up in `hosts`.
#[tauri::command]
fn hosts_check() -> Result<(), daemon::Error> {
    daemon::request(Request::HostsCheck).map(|_| ())
}

/// Engine, Lua runtime, nftables, distro, and any zapret2 install already present. Local, so
/// onboarding works before any daemon exists.
#[tauri::command]
fn detect_environment() -> detect::Environment {
    detect::environment()
}

/// Downloads the pinned zapret2 release and installs it after a polkit or UAC prompt. Blocks for the
/// download and for as long as the prompt is open, so it runs on a pool thread.
#[tauri::command]
async fn install_engine() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(engine_install::install)
        .await
        .map_err(|e| format!("the install did not finish: {e}"))?
}

/// Backs up an existing zapret2 install's config and imports it as a saved config.
#[tauri::command]
fn import_legacy() -> Result<configs::LegacyImport, String> {
    applog::logged(
        "importing the existing zapret2 config",
        configs::import_legacy(),
    )
}

/// Probes each host for a per-site verdict. Local only: nothing is uploaded. Runs on a pool thread
/// because an unreachable host costs a full timeout.
#[tauri::command]
async fn check_reachability(
    hosts: Option<Vec<String>>,
    timeout_ms: Option<u64>,
) -> Result<reachability::Report, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let hosts = hosts.unwrap_or_else(reachability::default_hosts);
        let timeout = timeout_ms.map_or(reachability::DEFAULT_TIMEOUT, Duration::from_millis);
        reachability::check(&hosts, timeout)
    })
    .await
    .map_err(|e| format!("the reachability check did not finish: {e}"))
}

/// The built-in sites, grouped, so the UI can show and switch off whole categories.
#[tauri::command]
fn site_categories() -> &'static [reachability::Category] {
    reachability::CATEGORIES
}

/// The strategies auto-configuration tries, in order. The walk runs in the UI: each step is an
/// ordinary start, check and stop, and the trial run's own deadline restores the machine if the
/// window dies.
#[tauri::command]
fn autoconfig_candidates() -> Vec<candidates::Candidate> {
    candidates::ranked()
}

/// What in this config will not do what it looks like it does. Local, so the UI can lint as you type.
#[tauri::command]
fn lint_config(config: Config) -> Vec<Warning> {
    registry::lint(&config)
}

/// The Lua functions this build can offer in a strategy editor.
#[tauri::command]
fn known_functions() -> Vec<&'static str> {
    registry::FUNCTIONS.iter().map(|f| f.name).collect()
}

/// A working config to start from, used by the first-run wizard.
#[tauri::command]
fn starter_config(name: String) -> Config {
    blkbstr_core::render::starter_config(&name)
}

/// Exactly what the daemon will hand the engine.
#[tauri::command]
fn preview_config(config: Config) -> Result<String, String> {
    let platform =
        blkbstr_core::Platform::current().ok_or("this platform has no zapret2 engine")?;
    let mut options = blkbstr_core::render::EngineOptions::new(platform);
    options.auto_hostlist = Some(blkbstr_core::paths::for_engine(
        &blkbstr_core::paths::auto_hostlist(),
    ));
    options.manual_hostlist = Some(blkbstr_core::paths::for_engine(
        &blkbstr_core::paths::manual_hostlist(),
    ));
    Ok(blkbstr_core::render::parameter_file(&config, &options))
}

#[tauri::command]
fn list_logs() -> Result<Vec<logs::LogFile>, String> {
    logs::list()
}

#[tauri::command]
fn read_log(name: String) -> Result<String, String> {
    logs::tail(&name)
}

/// Writes a diagnostics file and returns its path. Not uploaded: engine logs name the hosts they saw.
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

/// Deletes the file only. If that config is running it keeps running; stopping is a separate decision.
#[tauri::command]
fn delete_config(name: String) -> Result<(), String> {
    configs::delete(&name)
}

/// WebKitGTK's DMA-BUF renderer fails on the NVIDIA proprietary driver under Wayland, killing the
/// process at startup with:
///
/// ```text
/// Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.
/// ```
///
/// Disabled unless the user set the variable themselves; a slower renderer beats a window that
/// never opens.
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

/// Registers the background helper. Windows only: it raises one UAC prompt.
#[tauri::command]
async fn service_install() -> Result<(), String> {
    tracing::info!("installing the background helper");
    let result = tauri::async_runtime::spawn_blocking(service::install)
        .await
        .map_err(|e| format!("the install did not finish: {e}"))?;
    applog::logged("installing the background helper", result)
}

/// Both prompt (polkit, or UAC for the start type); they fail rather than hang when no agent is running.
#[tauri::command]
fn service_set_enabled(enabled: bool) -> Result<(), String> {
    applog::logged(
        "changing the start-at-boot setting",
        service::set_enabled(enabled),
    )
}

#[tauri::command]
fn service_set_active(active: bool) -> Result<(), String> {
    applog::logged(
        "starting or stopping the service",
        service::set_active(active),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    work_around_webkit_dmabuf();

    let _log = applog::init();
    tauri::Builder::default()
        // A second launch while the first sits in the tray brings that window back instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show(app);
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            // Closing the window leaves the app in the tray; "Quit" in the tray menu ends it.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" && tray::present() {
                    api.prevent_close();
                    window.hide().ok();
                }
            }
        })
        .setup(|app| {
            update_watch::spawn(app.handle().clone());
            // No tray (no appindicator on this desktop) is not worth refusing to start over.
            if let Err(e) = tray::build(app.handle()) {
                eprintln!("no tray icon: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            daemon_info,
            detect_environment,
            install_engine,
            import_legacy,
            check_reachability,
            site_categories,
            autoconfig_candidates,
            engine_status,
            engine_start,
            engine_confirm,
            engine_stop,
            hosts,
            hosts_add,
            hosts_remove,
            hosts_check,
            lint_config,
            known_functions,
            starter_config,
            preview_config,
            service_status,
            service_install,
            service_set_enabled,
            service_set_active,
            list_logs,
            read_log,
            export_diagnostics,
            list_configs,
            load_config,
            save_config,
            delete_config,
            updater::check_app_update,
            updater::install_app_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
