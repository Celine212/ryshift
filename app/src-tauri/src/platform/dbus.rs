//! Linux: a small D-Bus service the GNOME Shell extension talks to.
//!
//! On Wayland only the compositor sees global input, so the extension (which
//! runs inside GNOME Shell) spots "file drag + Shift" and calls these methods.
//! It also moves the wheel window to the pointer, which a Wayland app can't do
//! for itself. The drop itself still lands on our window as a normal drop.
//!
//! Kept free of Tauri types so it can be built and tested on its own.

pub const BUS_NAME: &str = "io.github.alexriderspy.Ryshift";
pub const OBJECT_PATH: &str = "/io/github/alexriderspy/Ryshift";

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    /// Open the wheel; (x, y) is the pointer in logical screen pixels.
    Show { x: f64, y: f64, shortcut: bool },
    /// Shift tapped again: next wheel.
    Cycle,
    /// The mouse button came up.
    MouseUp,
}

struct Service {
    on: Box<dyn Fn(Request) + Send + Sync>,
}

#[zbus::interface(name = "io.github.alexriderspy.Ryshift")]
impl Service {
    /// `mode` is "drag" or "shortcut".
    fn show(&self, x: f64, y: f64, mode: &str) {
        (self.on)(Request::Show { x, y, shortcut: mode == "shortcut" });
    }

    fn cycle(&self) {
        (self.on)(Request::Cycle);
    }

    fn mouse_up(&self) {
        (self.on)(Request::MouseUp);
    }
}

/// Claim the bus name and serve requests for the rest of the app's life.
pub fn serve(on: impl Fn(Request) + Send + Sync + 'static) -> zbus::Result<()> {
    let connection = zbus::blocking::connection::Builder::session()?
        .name(BUS_NAME)?
        .serve_at(OBJECT_PATH, Service { on: Box::new(on) })?
        .build()?;
    // The connection runs its own thread; keep it open until the app exits.
    std::mem::forget(connection);
    Ok(())
}
