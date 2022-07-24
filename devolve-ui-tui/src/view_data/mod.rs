//! Views for the TUI renderer. Can also be used in other renderers,
//! though you will need to extend to support video or advanced graphics or anything else TUIs don't support.

pub mod attrs;
pub mod constr;
#[cfg(feature = "images")]
pub mod terminal_image;
pub mod tui;
