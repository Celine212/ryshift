// Release builds on Windows shouldn't open a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;

use serde::Serialize;
use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, PhysicalPosition, Position};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// ⌥ Space on Mac. Alt + Space is the window menu on Windows and GNOME, so
/// those get Ctrl + Alt + Space.
#[cfg(target_os = "macos")]
const SHORTCUT: (Modifiers, &str) = (Modifiers::ALT, "⌥ Space");
#[cfg(not(target_os = "macos"))]
const SHORTCUT: (Modifiers, &str) = (Modifiers::CONTROL.union(Modifiers::ALT), "Ctrl + Alt + Space");

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
    /// Drag positions from the web view are labelled physical pixels, but only
    /// Windows really sends pixels; macOS and Linux send points. The UI
    /// divides by the pixel ratio only when this is true.
    drop_in_pixels: bool,
}

/// Scale factor of the monitor containing a physical screen point.
fn scale_at(app: &AppHandle, x: f64, y: f64) -> f64 {
    let monitors = app.available_monitors().unwrap_or_default();
    monitors
        .iter()
        .find(|m| {
            let (p, s) = (m.position(), m.size());
            x >= p.x as f64 && y >= p.y as f64 && x < (p.x + s.width as i32) as f64 && y < (p.y + s.height as i32) as f64
        })
        .map(|m| m.scale_factor())
        .unwrap_or(1.0)
}

/// Open the wheel centred on `at` (from `platform::cursor`) for these files.
pub fn show_wheel(app: &AppHandle, paths: Vec<PathBuf>, at: Position, mode: &'static str) {
    let Some(win) = app.get_webview_window("wheel") else { return };
    let payload = ShowPayload {
        actions: engine::actions_for(&paths),
        size: engine::total_size(&paths),
        paths: paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        mode,
        drop_in_pixels: cfg!(target_os = "windows"),
    };
    let half = WHEEL_SIZE / 2.0;
    let top_left: Position = match at {
        Position::Logical(p) => LogicalPosition::new(p.x - half, p.y - half).into(),
        Position::Physical(p) => {
            let (x, y) = (p.x as f64, p.y as f64);
            let half = half * scale_at(app, x, y);
            PhysicalPosition::new((x - half) as i32, (y - half) as i32).into()
        }
    };
    let _ = win.set_position(top_left);
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
            show_wheel(&handle, paths, platform::cursor(), "shortcut");
        });
    });
}

fn tray(app: &tauri::App) -> tauri::Result<()> {
    let hint = MenuItem::with_id(app, "hint", "Drag a file and press Shift", false, None::<&str>)?;
    let label = format!("Or select a file and press {}", SHORTCUT.1);
    let shortcut = MenuItem::with_id(app, "shortcut", label, false, None::<&str>)?;
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
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
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
            // Another app may already own the shortcut; dragging still works then.
            let shortcut = Shortcut::new(Some(SHORTCUT.0), Code::Space);
            if let Err(e) = app.global_shortcut().register(shortcut) {
                eprintln!("Shiftwheel: couldn't register {}: {e}", SHORTCUT.1);
            }
            if let Some(win) = app.get_webview_window("wheel") {
                platform::prepare(&win);
            }
            platform::watch_drags(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Shiftwheel");
}
