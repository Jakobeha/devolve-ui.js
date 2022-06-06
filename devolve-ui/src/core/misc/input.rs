use bitflags::bitflags;
use crate::core::view::layout::geom::{Pos, Size};

///! A lot of this is taken straight from crossterm's event data structures:
///! https://docs.rs/crossterm/0.23.2/src/crossterm/event.rs.html#297-413
///! Generalized to support non-terminal environments and platform-specific actions,
///! although not every environment / platform will support those

/// Represents an event.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Event {
    /// A single key event with additional pressed modifiers.
    Key(KeyEvent),
    /// A single mouse event with additional pressed modifiers.
    Mouse(MouseEvent),
    /// A single window or column resize event.
    Resize(ResizeEvent)
}

/// Represents a mouse event.
///
/// # Platform-specific Notes
///
/// ## Mouse Buttons
///
/// Some platforms/terminals do not report mouse button for the
/// `MouseEventKind::Up` and `MouseEventKind::Drag` events. `MouseButton::Left`
/// is returned if we don't know which button was used.
///
/// ## Key Modifiers
///
/// Some platforms/terminals does not report all key modifiers
/// combinations for all mouse event types. For example - macOS reports
/// `Ctrl` + left mouse button click as a right mouse button click.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MouseEvent {
    /// The kind of mouse event that was caused.
    pub kind: MouseEventKind,
    /// The position of the mouse cursor.
    /// Uses devolve-ui's coordinate system, AKA the pixel position is divided by column_size
    pub pos: Pos,
    /// The key modifiers active when the event occurred.
    pub modifiers: KeyModifiers,
}

/// A mouse event kind.
///
/// # Platform-specific Notes
///
/// ## Mouse Buttons
///
/// Some platforms/terminals do not report mouse button for the
/// `MouseEventKind::Up` and `MouseEventKind::Drag` events. `MouseButton::Left`
/// is returned if we don't know which button was used.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub enum MouseEventKind {
    /// Pressed mouse button. Contains the button that was pressed.
    Down(MouseButton),
    /// Released mouse button. Contains the button that was released.
    Up(MouseButton),
    /// Moved the mouse cursor while pressing the contained mouse button.
    Drag(MouseButton),
    /// Moved the mouse cursor while not pressing a mouse button.
    Moved,
    /// Scrolled mouse wheel downwards (towards the user).
    ScrollDown,
    /// Scrolled mouse wheel upwards (away from the user).
    ScrollUp,
}

/// Represents a mouse button.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button.
    Middle,
}

bitflags! {
    /// Represents key modifiers (shift, control, alt, meta).
    /// 'meta' key is the Command key on macOS, and the Windows key on Windows.
    pub struct KeyModifiers: u8 {
        const SHIFT = 0b0000_0001;
        const CONTROL = 0b0000_0010;
        const ALT = 0b0000_0100;
        const META = 0b0000_1000;
        const NONE = 0b0000_0000;
    }
}

impl KeyModifiers {
    /// The META (Command) key on macOS, and control key on other platforms.
    #[cfg(target_os = "macos")]
    pub const MACOS_CTRL: Self = Self::META;
    #[cfg(not(target_os = "macos"))]
    pub const MACOS_CTRL: Self = Self::CONTROL;
}

/// Represents a key event.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub struct KeyEvent {
    /// The key itself.
    code: KeyCode,
    /// Additional key modifiers.
    modifiers: KeyModifiers,
}

impl KeyEvent {
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent { code, modifiers }
    }
}

impl From<KeyCode> for KeyEvent {
    fn from(code: KeyCode) -> Self {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }
}

/// Represents a key.
#[derive(Debug, PartialOrd, PartialEq, Eq, Clone, Copy, Hash)]
pub enum KeyCode {
    /// Backspace key.
    Backspace,
    /// Enter key.
    Enter,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page up key.
    PageUp,
    /// Page down key.
    PageDown,
    /// Tab key.
    Tab,
    /// Shift + Tab key.
    BackTab,
    /// Delete key.
    Delete,
    /// Insert key.
    Insert,
    /// F key.
    ///
    /// `KeyCode::F(1)` represents F1 key, etc.
    F(u8),
    /// A character.
    ///
    /// The char is always lowercase and the `shift` modifier determines if the char is shift.
    /// `KeyEvent::char()` will return withe the proper case.
    ///
    /// e.g. `KeyCode::Char('c')` represents `c` or `C` character
    CharAsLowercase(char),
    /// Null.
    Null,
    /// Escape key.
    Esc,
    /// Other keycode
    Other(u32)
}

impl KeyEvent {
    /// If the event is a char, returns it with proper case
    pub fn char(&self) -> Option<char> {
        match self.code {
            KeyCode::CharAsLowercase(c) => Some(if self.modifiers.contains(KeyModifiers::SHIFT) {
                c.to_uppercase().next().unwrap()
            } else {
                c
            }),
            _ => None
        }
    }
}

/// Represents a window or column resize event
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ResizeEvent {
    /// The new size, in devolve-ui coordinates
    Window(Size),
    /// The new column size in pixels
    Column(Size)
}