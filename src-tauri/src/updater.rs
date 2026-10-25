//! App updates from GitHub Releases. The engine is updated separately (`install_engine`): it runs
//! as root and has its own pinned hash, so it never rides along with an app update.

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
pub struct AppUpdate {
    version: String,
    notes: Option<String>,
    /// False for installs the updater cannot replace in place (deb, rpm): the user gets the
    /// version and updates through the package they installed from.
    installable: bool,
}

/// Linux packages are owned by the package manager; only an AppImage can overwrite itself.
fn installable() -> bool {
    cfg!(windows) || (cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_some())
}

pub async fn newer(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    app.updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> Result<Option<AppUpdate>, String> {
    let found = crate::applog::logged("checking for an app update", newer(&app).await)?;
    tracing::info!(newer = ?found.as_ref().map(|u| &u.version), "checked for an app update");
    Ok(found.map(|u| AppUpdate {
        version: u.version,
        notes: u.body,
        installable: installable(),
    }))
}

/// Downloads, verifies the signature against the key in `tauri.conf.json`, installs and restarts.
#[tauri::command]
pub async fn install_app_update(app: AppHandle) -> Result<(), String> {
    if !installable() {
        return Err("this install is updated through its package, not from the app".into());
    }
    let update = newer(&app).await?.ok_or("already up to date")?;
    tracing::info!(version = %update.version, "installing the app update");
    crate::applog::logged(
        "installing the app update",
        update
            .download_and_install(|_, _| {}, || {})
            .await
            .map_err(|e| e.to_string()),
    )?;
    app.restart()
}
