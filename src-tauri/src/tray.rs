//! Tray icon: engine state at a glance, a toggle, and a way back to the window. The daemon does
//! not push, so a thread polls it; the window's own poll is separate.

use crate::{configs, daemon};
use blkbstr_core::protocol::{EngineStatus, Request, Response};
use std::sync::Mutex;
use std::time::Duration;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

const ID: &str = "main";
const POLL: Duration = Duration::from_secs(5);

struct Items {
    status: MenuItem<tauri::Wry>,
    toggle: MenuItem<tauri::Wry>,
    on: Image<'static>,
    off: Image<'static>,
    /// The config the engine last ran, so "Turn on" brings back what the user had.
    last: Mutex<Option<String>>,
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Off", false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Turn on", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .item(&status)
        .item(&toggle)
        .separator()
        .text("show", "Show Blockbuster")
        .text("quit", "Quit")
        .build()?;

    let on = app
        .default_window_icon()
        .ok_or(tauri::Error::WindowNotFound)?
        .clone()
        .to_owned();
    let off = dimmed(&on);
    TrayIconBuilder::with_id(ID)
        .icon(off.clone())
        .tooltip("Blockbuster")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => {
                let app = app.clone();
                std::thread::spawn(move || toggle_engine(&app));
            }
            "show" => show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    app.manage(Items {
        status,
        toggle,
        on,
        off,
        last: Mutex::new(None),
    });

    let app = app.clone();
    std::thread::spawn(move || loop {
        refresh(&app);
        std::thread::sleep(POLL);
    });
    Ok(())
}

fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        window.show().ok();
        window.unminimize().ok();
        window.set_focus().ok();
    }
}

fn status() -> Result<EngineStatus, String> {
    match daemon::request(Request::Status) {
        Ok(Response::Status(s)) => Ok(s),
        Ok(other) => Err(format!("unexpected reply: {other:?}")),
        Err(e) => Err(e.to_string()),
    }
}

fn toggle_engine(app: &AppHandle) {
    let running = status().is_ok_and(|s| s.running);
    let result = if running {
        daemon::request(Request::Stop)
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        start_last(app)
    };
    match result {
        Ok(()) => refresh(app),
        Err(e) => {
            let items = app.state::<Items>();
            items.status.set_text(format!("Failed: {e}")).ok();
        }
    }
}

fn start_last(app: &AppHandle) -> Result<(), String> {
    let remembered = app.state::<Items>().last.lock().unwrap().clone();
    let name = match remembered {
        Some(n) => n,
        None => configs::list()?
            .into_iter()
            .next()
            .ok_or("no saved config")?,
    };
    let config = configs::load(&name)?;
    daemon::request(Request::Start {
        config: Box::new(config),
        ephemeral: false,
    })
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn refresh(app: &AppHandle) {
    let Some(items) = app.try_state::<Items>() else {
        return;
    };
    let (label, running) = match status() {
        Ok(s) if s.running => {
            let name = s.active_config.clone().unwrap_or_default();
            *items.last.lock().unwrap() = s.active_config;
            let trial = s.revert_in_seconds.is_some();
            (
                format!("{} {name}", if trial { "Trying" } else { "On:" }),
                true,
            )
        }
        Ok(_) => ("Off".into(), false),
        Err(_) => ("Helper not running".into(), false),
    };
    items.status.set_text(label).ok();
    items
        .toggle
        .set_text(if running { "Turn off" } else { "Turn on" })
        .ok();
    if let Some(tray) = app.tray_by_id(ID) {
        tray.set_icon(Some(if running {
            items.on.clone()
        } else {
            items.off.clone()
        }))
        .ok();
    }
}

/// Grey and half transparent: "off" reads without a colour.
fn dimmed(icon: &Image<'_>) -> Image<'static> {
    let mut rgba = icon.rgba().to_vec();
    for i in (0..rgba.len()).step_by(4) {
        let sum = u16::from(rgba[i]) + u16::from(rgba[i + 1]) + u16::from(rgba[i + 2]);
        rgba[i..i + 3].fill((sum / 3) as u8);
        rgba[i + 3] /= 2;
    }
    Image::new_owned(rgba, icon.width(), icon.height())
}
