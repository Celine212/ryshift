//! macOS: watch the system drag pasteboard and the Shift key.
//!
//! When any app starts a drag, macOS bumps the change count of the shared
//! "drag" pasteboard. If that happens while the mouse button is held and the
//! pasteboard holds file URLs, a file is being dragged. These are all plain
//! reads of global state, so no Accessibility permission is needed.

use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSEvent, NSEventModifierFlags, NSGlassEffectView, NSPasteboard,
    NSPasteboardNameDrag, NSScreen, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowOrderingMode,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString, NSURL};
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
                crate::debug!("drag started: {} file(s), shift already held: {}", st.paths.len(), shift);
            }
            if st.dragging && shift && !st.shift {
                crate::debug!("shift pressed mid-drag (wheel shown: {})", st.shown);
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

/// Window is 320pt square; the glass disk is the wheel's 300pt circle.
const DISK_INSET: f64 = 10.0;
const DISK_SIZE: f64 = 300.0;

/// Put real glass behind the (transparent) web view, cut to the wheel's circle.
/// macOS 26+ gets Liquid Glass, which refracts what's underneath; older
/// versions get the standard frosted HUD blur.
fn add_glass(w: &NSWindow) {
    let (Some(mtm), Some(content)) = (MainThreadMarker::new(), w.contentView()) else { return };
    let frame = NSRect::new(NSPoint::new(DISK_INSET, DISK_INSET), NSSize::new(DISK_SIZE, DISK_SIZE));
    let radius = DISK_SIZE / 2.0;
    let view: objc2::rc::Retained<NSView> = if AnyClass::get(c"NSGlassEffectView").is_some() {
        let glass = NSGlassEffectView::initWithFrame(NSGlassEffectView::alloc(mtm), frame);
        glass.setCornerRadius(radius);
        objc2::rc::Retained::into_super(glass)
    } else {
        let blur = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), frame);
        blur.setMaterial(NSVisualEffectMaterial::HUDWindow);
        blur.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        blur.setState(NSVisualEffectState::Active);
        blur.setWantsLayer(true);
        unsafe {
            let layer: *mut AnyObject = msg_send![&*blur, layer];
            if !layer.is_null() {
                let _: () = msg_send![layer, setCornerRadius: radius];
                let _: () = msg_send![layer, setMasksToBounds: true];
            }
        }
        objc2::rc::Retained::into_super(blur)
    };
    view.setAutoresizingMask(NSAutoresizingMaskOptions::empty());
    content.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, None);
}

/// The wheel sits on real OS glass here, so the UI skips its CSS stand-in.
pub fn native_glass() -> bool {
    true
}

/// Float the wheel above everything, on every Space and over full-screen apps.
pub fn prepare(win: &WebviewWindow) {
    if let Some(w) = ns_window(win) {
        add_glass(w);
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
pub fn present(win: &WebviewWindow, top_left: Position, focus: bool) {
    place(win, top_left);
    if focus {
        let _ = win.show();
        let _ = win.set_focus();
    } else if let Some(w) = ns_window(win) {
        w.orderFrontRegardless();
        crate::debug!("wheel ordered front: visible {}, frame {:?}", w.isVisible(), w.frame());
    } else {
        crate::debug!("present: no NSWindow");
    }
}

fn primary_screen_height() -> f64 {
    MainThreadMarker::new()
        .and_then(|mtm| NSScreen::screens(mtm).firstObject())
        .map(|s| s.frame().size.height)
        .unwrap_or(0.0)
}

/// Move the window now, before it's shown. AppKit's origin is the bottom-left
/// of the main screen, so flip y.
fn place(win: &WebviewWindow, top_left: Position) {
    let (x, y) = match top_left {
        Position::Logical(p) => (p.x, p.y),
        Position::Physical(p) => {
            let s = win.scale_factor().unwrap_or(1.0);
            (p.x as f64 / s, p.y as f64 / s)
        }
    };
    if let Some(w) = ns_window(win) {
        w.setFrameTopLeftPoint(NSPoint::new(x, primary_screen_height() - y));
    }
}

/// Mouse position in top-left based screen points. Must run on the main thread.
pub fn cursor() -> Position {
    let p = NSEvent::mouseLocation();
    LogicalPosition::new(p.x, primary_screen_height() - p.y).into()
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
