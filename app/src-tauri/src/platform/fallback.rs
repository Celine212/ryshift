//! Windows and Linux until their drag watchers land (milestones M2 and M3).
//! The app still builds and runs there: the shortcut opens the wheel at the
//! mouse, and files dropped onto the open wheel are processed.

use std::path::PathBuf;
use tauri::{AppHandle, WebviewWindow};

pub fn prepare(_win: &WebviewWindow) {}

pub fn watch_drags(_app: AppHandle) {}

pub fn present(win: &WebviewWindow, _focus: bool) {
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn cursor() -> (f64, f64) {
    (400.0, 300.0)
}

pub fn selected_files() -> Vec<PathBuf> {
    Vec::new()
}
