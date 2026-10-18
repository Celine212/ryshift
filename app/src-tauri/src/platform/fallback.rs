//! Desktops without a drag watcher yet (Ubuntu on Wayland until the GNOME
//! extension lands in M3). The app still runs: the shortcut opens the wheel,
//! and files dropped onto the open wheel are processed.

use std::path::PathBuf;
use tauri::{AppHandle, PhysicalPosition, Position, WebviewWindow};

pub fn prepare(_win: &WebviewWindow) {}

pub fn watch_drags(_app: AppHandle) {}

pub fn present(win: &WebviewWindow, _focus: bool) {
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn cursor() -> Position {
    PhysicalPosition::new(400, 300).into()
}

pub fn selected_files() -> Vec<PathBuf> {
    Vec::new()
}

pub fn native_glass() -> bool {
    false
}
