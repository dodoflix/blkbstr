//! Tells the user about a new app version while the window is closed to the tray, where the toast
//! the window shows would never be seen. With the window up the toast is enough.

use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        let mut announced: Option<String> = None;
        loop {
            match tauri::async_runtime::block_on(crate::updater::newer(&app)) {
                Ok(Some(update)) if announced.as_deref() != Some(&update.version) => {
                    announced = Some(update.version.clone());
                    if !window_in_use(&app) {
                        notify(&app, &update.version);
                    }
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "background update check failed"),
            }
            std::thread::sleep(EVERY);
        }
    });
}

fn window_in_use(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false))
}

fn notify(app: &AppHandle, version: &str) {
    tracing::info!(version, "telling the user about an app update");
    let shown = app
        .notification()
        .builder()
        .title(format!("Blockbuster {version} is available"))
        .body("Open Blockbuster, then System, to install it.")
        .show();
    if let Err(e) = shown {
        tracing::warn!(error = %e, "could not show the update notification");
    }
}
