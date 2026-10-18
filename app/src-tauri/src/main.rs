// Release builds on Windows shouldn't open a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;

use serde::Serialize;
use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalPosition, Manager};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

/// The wheel window is a square this many points wide, centred on the cursor.
const WHEEL_SIZE: f64 = 320.0;

#[derive(Clone, Serialize)]
struct ShowPayload {
    paths: Vec<String>,
    actions: Vec<engine::ActionInfo>,
    size: u64,
    /// "drag": opened mid-drag, the file arrives by drop.
    /// "shortcut": opened from the selection, slices are clicked.
    mode: &'static str,
}

/// Open the wheel at (x, y), top-left based screen points, for these files.
pub fn show_wheel(app: &AppHandle, paths: Vec<PathBuf>, x: f64, y: f64, mode: &'static str) {
    let Some(win) = app.get_webview_window("wheel") else { return };
    let payload = ShowPayload {
        actions: engine::actions_for(&paths),
        size: engine::total_size(&paths),
        paths: paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        mode,
    };
    let half = WHEEL_SIZE / 2.0;
    let _ = win.set_position(LogicalPosition::new(x - half, y - half));
    let _ = win.emit("wheel:show", payload);
    platform::present(&win, mode == "shortcut");
}

#[tauri::command]
async fn run_action(id: String, paths: Vec<String>) -> Result<Vec<String>, String> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || engine::run(&id, &paths))
        .await
        .map_err(|e| e.to_string())?
        .map(|out| out.iter().map(|p| p.to_string_lossy().into_owned()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_wheel(app: AppHandle) {
    if let Some(win) = app.get_webview_window("wheel") {
        let _ = win.hide();
    }
}

/// Select files, press the shortcut, and the wheel opens at the mouse.
fn open_from_selection(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let paths = platform::selected_files();
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let (x, y) = platform::cursor();
            show_wheel(&handle, paths, x, y, "shortcut");
        });
    });
}

fn tray(app: &tauri::App) -> tauri::Result<()> {
    let hint = MenuItem::with_id(app, "hint", "Drag a file and press Shift", false, None::<&str>)?;
    let shortcut = MenuItem::with_id(app, "shortcut", "Or select a file and press ⌥ Space", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Shiftwheel", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(app, &[&hint, &shortcut, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Shiftwheel")
        .menu(&menu)
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn main() {
    let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut(shortcut)
                .expect("valid shortcut")
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        open_from_selection(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![run_action, hide_wheel])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            tray(app)?;
            if let Some(win) = app.get_webview_window("wheel") {
                platform::prepare(&win);
            }
            platform::watch_drags(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Shiftwheel");
}
