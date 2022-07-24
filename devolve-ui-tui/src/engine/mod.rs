//! Tui render engine. Makes the renderer draw TUI data to STDOUT or another provided socket.
//!
//! You can choose to omit the TUI escape codes if you want (e.g. to create something you pipe to another process),
//! though that will limit the functionality (TODO implement this)

mod layer;
#[cfg(feature = "tui-images")]
mod terminal_image;
pub mod tui;