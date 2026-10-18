#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

macro_rules! debug {
    ($($arg:tt)*) => {
        if std::env::var_os("RYSHIFT_DEBUG").is_some() {
            eprintln!("[ryshift] {}", format!($($arg)*));
        }
    };
}
#[allow(unused_imports)]
pub(crate) use debug;

mod platform;

use serde::Serialize;
use std::path::PathBuf;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use base64::Engine as _;
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, PhysicalPosition, Position, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg(target_os = "macos")]
const SHORTCUT: (Modifiers, &str) = (Modifiers::ALT, "⌥ Space");
#[cfg(not(target_os = "macos"))]
const SHORTCUT: (Modifiers, &str) = (Modifiers::CONTROL.union(Modifiers::ALT), "Ctrl + Alt + Space");

const WHEEL_SIZE: f64 = 320.0;

#[derive(Clone, Serialize)]
struct ShowPayload {
    paths: Vec<String>,
    actions: Vec<engine::ActionInfo>,
    size: u64,
    mode: &'static str,
    drop_in_pixels: bool,
    native_glass: bool,
}

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

pub fn show_wheel(app: &AppHandle, paths: Vec<PathBuf>, at: Position, mode: &'static str) {
    let Some(win) = app.get_webview_window("wheel") else {
        debug!("show_wheel: no wheel window");
        return;
    };
    debug!("show_wheel: {} file(s), mode {mode}, at {at:?}", paths.len());
    let payload = ShowPayload {
        actions: engine::actions_for(&paths),
        size: engine::total_size(&paths),
        paths: paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        mode,
        drop_in_pixels: cfg!(target_os = "windows"),
        native_glass: platform::native_glass(),
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
    platform::present(&win, top_left, mode == "shortcut");
}

#[derive(Serialize)]
struct Slices {
    actions: Vec<engine::ActionInfo>,
    size: u64,
}

#[tauri::command]
fn actions_for(paths: Vec<String>) -> Slices {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    Slices { actions: engine::actions_for(&paths), size: engine::total_size(&paths) }
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
fn open_panel(app: AppHandle, panel: String, path: String) -> Result<(), String> {
    let title = match panel.as_str() {
        "crop" => "Crop image",
        "edit" => "Edit image",
        "annotate" => "Annotate",
        "addbg" => "Add background",
        "redact" => "Redact",
        _ => return Err(format!("unknown panel {panel}")),
    };
    static N: AtomicU32 = AtomicU32::new(0);
    let label = format!("editor-{}", N.fetch_add(1, Ordering::Relaxed));
    let init = format!("window.__RYSHIFT_EDIT__ = {};", serde_json::json!({ "mode": panel, "path": path }));
    let win = WebviewWindowBuilder::new(&app, label, WebviewUrl::App("editor.html".into()))
        .title(title)
        .inner_size(900.0, 660.0)
        .min_inner_size(600.0, 460.0)
        .center()
        .initialization_script(&init)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = win.set_focus();
    Ok(())
}

#[derive(Serialize)]
struct EditImage {
    data_url: String,
    width: u32,
    height: u32,
    ext: &'static str,
    name: String,
}

#[tauri::command]
async fn load_for_edit(path: String) -> Result<EditImage, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(path);
        let src = engine::load_for_edit(&path).map_err(|e| e.to_string())?;
        Ok(EditImage {
            data_url: format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&src.png)),
            width: src.width,
            height: src.height,
            ext: src.out_ext,
            name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn save_edit(path: String, suffix: String, ext: String, data: String) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || engine::save_edit(&PathBuf::from(path), &suffix, &ext, &bytes))
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_wheel(app: AppHandle) {
    if let Some(win) = app.get_webview_window("wheel") {
        let _ = win.hide();
    }
}

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
    let quit = MenuItem::with_id(app, "quit", "Quit Ryshift", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(app, &[&hint, &shortcut, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Ryshift")
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
        .invoke_handler(tauri::generate_handler![actions_for, run_action, hide_wheel, open_panel, load_for_edit, save_edit])
        .setup(|app| {
            if std::env::var_os("RYSHIFT_PDFIUM").is_none() {
                if let Ok(dir) = app.path().resource_dir() {
                    std::env::set_var("RYSHIFT_PDFIUM", dir.join("lib"));
                }
            }
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            tray(app)?;
            let shortcut = Shortcut::new(Some(SHORTCUT.0), Code::Space);
            if let Err(e) = app.global_shortcut().register(shortcut) {
                eprintln!("Ryshift: couldn't register {}: {e}", SHORTCUT.1);
            }
            if let Some(win) = app.get_webview_window("wheel") {
                platform::prepare(&win);
            }
            platform::watch_drags(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Ryshift");
}
