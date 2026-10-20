use std::path::PathBuf;
use std::time::Duration;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Position, WebviewWindow};
use windows::core::Interface;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{CreateEllipticRgn, RedrawWindow, SetWindowRgn, RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE, RDW_UPDATENOW};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, IDispatch, IServiceProvider, CLSCTX_ALL,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_SHIFT};
use windows::Win32::UI::Shell::{
    IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp, SID_STopLevelBrowser,
    ShellWindows, SIGDN_FILESYSPATH, SVGIO_SELECTION, SWC_DESKTOP, SWFO_NEEDDISPATCH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetAncestor, GetClassNameW, GetCursorPos, GetForegroundWindow, GetSystemMetrics, GetWindowRect, SetWindowPos,
    ShowWindow, WindowFromPoint, GA_ROOT, HWND_TOPMOST, SM_CXDRAG, SM_CYDRAG, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE,
};

const CSIDL_DESKTOP: i32 = 0;

fn variant_i32(v: i32) -> VARIANT {
    let mut var = VARIANT::default();
    unsafe {
        let inner = &mut *var.Anonymous.Anonymous;
        inner.vt = VT_I4;
        inner.Anonymous.lVal = v;
    }
    var
}

fn key_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

fn cursor_point() -> POINT {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    p
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

fn is_desktop(root: HWND) -> bool {
    matches!(class_name(root).as_str(), "Progman" | "WorkerW")
}

fn file_window_at(p: POINT) -> Option<HWND> {
    let root = unsafe { GetAncestor(WindowFromPoint(p), GA_ROOT) };
    let class = class_name(root);
    (class == "CabinetWClass" || is_desktop(root)).then_some(root)
}

#[derive(Default)]
struct DragState {
    mouse_down: bool,
    start: (i32, i32),
    source: Option<HWND>,
    dragging: bool,
    shown: bool,
    shift: bool,
    last: (i32, i32),
}

/// Where the cursor is, measured from the wheel window's top-left corner (physical pixels).
fn pointer_in_wheel(app: &AppHandle, p: POINT) -> Option<(i32, i32)> {
    let hwnd = hwnd_of(&app.get_webview_window("wheel")?)?;
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut r).ok()? };
    Some((p.x - r.left, p.y - r.top))
}

#[derive(Clone, serde::Serialize)]
struct Pointer {
    x: i32,
    y: i32,
}

pub fn watch_drags(app: AppHandle) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let (tx, ty) = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
        let mut st = DragState::default();
        loop {
            std::thread::sleep(Duration::from_millis(15));
            let down = key_down(VK_LBUTTON.0);
            let shift = key_down(VK_SHIFT.0);
            let p = cursor_point();

            if down && !st.mouse_down {
                st = DragState { mouse_down: true, start: (p.x, p.y), source: file_window_at(p), shift, ..Default::default() };
                continue;
            }
            if !down && st.mouse_down {
                if st.shown {
                    // Send where the button was released, so the wheel can run the slice
                    // under the cursor even when Windows (or another drag tool such as
                    // Huawei Superhub) never hands the drop to the wheel window.
                    let at = pointer_in_wheel(&app, p).map(|(x, y)| Pointer { x, y });
                    let _ = app.emit("wheel:mouseup", at);
                    let gen = crate::WHEEL_GEN.load(Ordering::SeqCst);
                    crate::hide_wheel_later(&app, gen, 1500);
                }
                st = DragState::default();
                continue;
            }
            if st.shown && (p.x, p.y) != st.last {
                // Drive the hover highlight from the real cursor position instead of
                // drag-over events, which some PCs don't deliver to the wheel.
                st.last = (p.x, p.y);
                if let Some((x, y)) = pointer_in_wheel(&app, p) {
                    let _ = app.emit("wheel:pointer", Pointer { x, y });
                }
            }
            if st.mouse_down && !st.dragging && st.source.is_some() {
                st.dragging = (p.x - st.start.0).abs() > tx || (p.y - st.start.1).abs() > ty;
            }
            if st.dragging && shift && !st.shift {
                if st.shown {
                    let _ = app.emit("wheel:cycle", ());
                } else if let Some(source) = st.source {
                    let paths = explorer_selection(source);
                    if !paths.is_empty() {
                        st.shown = true;
                        let handle = app.clone();
                        let at: Position = PhysicalPosition::new(p.x, p.y).into();
                        let _ = app.run_on_main_thread(move || crate::show_wheel(&handle, paths, at, "drag"));
                    }
                }
            }
            st.shift = shift;
        }
    });
}

fn explorer_selection(root: HWND) -> Vec<PathBuf> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let Ok(windows) = CoCreateInstance::<_, IShellWindows>(&ShellWindows, None, CLSCTX_ALL) else {
            return Vec::new();
        };
        let dispatch = if is_desktop(root) {
            let mut hwnd = 0i32;
            windows
                .FindWindowSW(&variant_i32(CSIDL_DESKTOP), &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH)
                .ok()
        } else {
            let count = windows.Count().unwrap_or(0);
            (0..count).find_map(|i| {
                let d = windows.Item(&variant_i32(i)).ok()?;
                let app: IWebBrowserApp = d.cast().ok()?;
                (app.HWND().ok()?.0 as isize == root.0 as isize).then_some(d)
            })
        };
        dispatch.and_then(|d| selection_of(&d).ok()).unwrap_or_default()
    }
}

unsafe fn selection_of(dispatch: &IDispatch) -> windows::core::Result<Vec<PathBuf>> {
    let provider: IServiceProvider = dispatch.cast()?;
    let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
    let view: IFolderView = browser.QueryActiveShellView()?.cast()?;
    let items: IShellItemArray = view.Items(SVGIO_SELECTION)?;
    let mut paths = Vec::new();
    for i in 0..items.GetCount()? {
        let name = items.GetItemAt(i)?.GetDisplayName(SIGDN_FILESYSPATH)?;
        paths.push(PathBuf::from(name.to_string()?));
        CoTaskMemFree(Some(name.0 as _));
    }
    Ok(paths)
}

fn hwnd_of(win: &WebviewWindow) -> Option<HWND> {
    win.hwnd().ok().map(|h| HWND(h.0 as _))
}

const DISK_INSET: f64 = 10.0;
const DISK_SIZE: f64 = 300.0;

pub fn prepare(_win: &WebviewWindow) {
    // Acrylic is left off: on some Windows 10 PCs it drew the wheel as a blank
    // gray square. The wheel's own translucent disk is used instead.
}

pub fn native_glass() -> bool {
    false
}

fn clip_to_disk(win: &WebviewWindow, hwnd: HWND) {
    let s = win.scale_factor().unwrap_or(1.0);
    let (a, b) = ((DISK_INSET * s) as i32, ((DISK_INSET + DISK_SIZE) * s) as i32 + 1);
    unsafe {
        let region = CreateEllipticRgn(a, a, b, b);
        let _ = SetWindowRgn(hwnd, Some(region), true);
    }
}

pub fn present(win: &WebviewWindow, top_left: Position, focus: bool) {
    let (x, y) = match top_left {
        Position::Physical(p) => (p.x, p.y),
        Position::Logical(p) => {
            let s = win.scale_factor().unwrap_or(1.0);
            ((p.x * s) as i32, (p.y * s) as i32)
        }
    };
    if let Some(hwnd) = hwnd_of(win) {
        clip_to_disk(win, hwnd);
        unsafe {
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }
    if focus {
        let _ = win.show();
        let _ = win.set_focus();
    } else if let Some(hwnd) = hwnd_of(win) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
    }
}

/// Hide the wheel without leaving a painted "ghost" of it behind: move it
/// off-screen, hide it, then ask Windows to repaint whatever was under it.
pub fn park_and_hide(win: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(win) else {
        let _ = win.hide();
        return;
    };
    let mut r = RECT::default();
    unsafe {
        let had_rect = GetWindowRect(hwnd, &mut r).is_ok();
        let _ = SetWindowPos(hwnd, None, -32000, -32000, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
        let _ = ShowWindow(hwnd, SW_HIDE);
        if had_rect {
            let _ = RedrawWindow(None, Some(&r as *const RECT), None, RDW_INVALIDATE | RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN | RDW_UPDATENOW);
        }
    }
    let _ = win.hide();
}

pub fn cursor() -> Position {
    let p = cursor_point();
    PhysicalPosition::new(p.x, p.y).into()
}

pub fn selected_files() -> Vec<PathBuf> {
    let root = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) };
    explorer_selection(root)
}
