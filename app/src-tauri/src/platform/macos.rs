//! macOS: watch the system drag pasteboard and the Shift key.
//!
//! When any app starts a drag, macOS bumps the change count of the shared
//! "drag" pasteboard. If that happens while the mouse button is held and the
//! pasteboard holds file URLs, a file is being dragged. These are all plain
//! reads of global state, so no Accessibility permission is needed.

use objc2_app_kit::{
    NSEvent, NSEventModifierFlags, NSPasteboard, NSPasteboardNameDrag, NSScreen, NSWindow,
    NSWindowCollectionBehavior,
};
use objc2_foundation::{MainThreadMarker, NSString, NSURL};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter, LogicalPosition, Position, WebviewWindow};

/// Above normal windows and the Dock, like a context menu.
const POP_UP_MENU_LEVEL: isize = 101;

#[derive(Default)]
struct DragState {
    mouse_down: bool,
    start_count: isize,
    dragging: bool,
    shown: bool,
    shift: bool,
    paths: Vec<PathBuf>,
}

fn file_urls(pb: &NSPasteboard) -> Vec<PathBuf> {
    let file_url = NSString::from_str("public.file-url");
    let Some(items) = pb.pasteboardItems() else { return Vec::new() };
    items
        .iter()
        .filter_map(|item| item.stringForType(&file_url))
        .filter_map(|s| NSURL::URLWithString(&s))
        .filter_map(|url| url.path())
        .map(|p| PathBuf::from(p.to_string()))
        .collect()
}

pub fn watch_drags(app: AppHandle) {
    std::thread::spawn(move || {
        let pb = unsafe { NSPasteboard::pasteboardWithName(NSPasteboardNameDrag) };
        let mut st = DragState::default();
        loop {
            std::thread::sleep(Duration::from_millis(15));
            let down = NSEvent::pressedMouseButtons() & 1 != 0;
            let shift = NSEvent::modifierFlags_class().contains(NSEventModifierFlags::Shift);
            let count = pb.changeCount();

            if down && !st.mouse_down {
                st = DragState { mouse_down: true, start_count: count, shift, ..Default::default() };
                continue;
            }
            if !down && st.mouse_down {
                if st.shown {
                    // The drop event reaches the wheel first; it hides itself
                    // after this unless it's busy converting.
                    let _ = app.emit("wheel:mouseup", ());
                }
                st = DragState::default();
                continue;
            }
            if st.mouse_down && !st.dragging && count != st.start_count {
                st.paths = file_urls(&pb);
                st.dragging = !st.paths.is_empty();
            }
            if st.dragging && shift && !st.shift {
                if st.shown {
                    let _ = app.emit("wheel:cycle", ());
                } else {
                    st.shown = true;
                    let (handle, paths) = (app.clone(), st.paths.clone());
                    let _ = app.run_on_main_thread(move || {
                        crate::show_wheel(&handle, paths, cursor(), "drag");
                    });
                }
            }
            st.shift = shift;
        }
    });
}

fn ns_window(win: &WebviewWindow) -> Option<&NSWindow> {
    let ptr = win.ns_window().ok()? as *const NSWindow;
    unsafe { ptr.as_ref() }
}

/// Float the wheel above everything, on every Space and over full-screen apps.
pub fn prepare(win: &WebviewWindow) {
    if let Some(w) = ns_window(win) {
        w.setLevel(POP_UP_MENU_LEVEL);
        w.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary,
        );
        w.setHidesOnDeactivate(false);
    }
}

/// Mid-drag the wheel must appear without stealing focus from Finder. From
/// the shortcut it takes focus so number keys and Esc work.
pub fn present(win: &WebviewWindow, focus: bool) {
    if focus {
        let _ = win.show();
        let _ = win.set_focus();
    } else if let Some(w) = ns_window(win) {
        w.orderFrontRegardless();
    }
}

/// Mouse position in top-left based screen points. Must run on the main thread.
pub fn cursor() -> Position {
    let p = NSEvent::mouseLocation();
    let height = MainThreadMarker::new()
        .and_then(|mtm| NSScreen::screens(mtm).firstObject())
        .map(|s| s.frame().size.height)
        .unwrap_or(0.0);
    LogicalPosition::new(p.x, height - p.y).into()
}

const FINDER_SELECTION: &str = r#"
set out to ""
tell application "Finder" to set sel to selection as alias list
repeat with f in sel
    set out to out & POSIX path of f & linefeed
end repeat
return out"#;

/// The files selected in Finder (or on the desktop).
pub fn selected_files() -> Vec<PathBuf> {
    let Ok(out) = Command::new("/usr/bin/osascript").args(["-e", FINDER_SELECTION]).output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| PathBuf::from(l.trim_end_matches('/')))
        .collect()
}
