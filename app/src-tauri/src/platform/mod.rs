//! The only OS-specific part: spotting "a file is being dragged and Shift was
//! pressed", placing the wheel above everything, and reading the selection.
//!
//! Each OS module provides the same five functions.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
mod fallback;
#[cfg(not(target_os = "macos"))]
pub use fallback::*;
