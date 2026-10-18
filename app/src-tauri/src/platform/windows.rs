use std::path::PathBuf;
use std::time::Duration;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{AppHandle, Emitter, PhysicalPosition, Position, WebviewWindow};
use windows::core::Interface;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{CreateEllipticRgn, SetWindowRgn};
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
    GetAncestor, GetClassNameW, GetCursorPos, GetForegroundWindow, GetSystemMetrics, SetWindowPos,
    ShowWindow, WindowFromPoint, GA_ROOT, HWND_TOPMOST, SM_CXDRAG, SM_CYDRAG, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
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
                    let _ = app.emit("wheel:mouseup", ());
                }
                st = DragState::default();
                continue;
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

pub fn prepare(win: &WebviewWindow) {
    let _ = win.set_effects(EffectsBuilder::new().effect(Effect::Acrylic).build());
}

pub fn native_glass() -> bool {
    true
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

pub fn cursor() -> Position {
    let p = cursor_point();
    PhysicalPosition::new(p.x, p.y).into()
}

pub fn selected_files() -> Vec<PathBuf> {
    let root = unsafe { GetAncestor(GetForegroundWindow(), GA_ROOT) };
    explorer_selection(root)
}
