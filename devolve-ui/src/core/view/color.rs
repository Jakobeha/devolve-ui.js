use std::fmt;
use std::fmt::{Debug, Formatter};
use palette::{Packed, Lcha, Srgba, IntoColor};
use palette::rgb::channels;

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Transparent,
    Srgba(Srgba),
    Lcha(Lcha),
}

#[derive(Clone, Copy)]
pub struct PackedColor(Packed<channels::Rgba>);

impl From<Color> for PackedColor {
    fn from(self_: Color) -> PackedColor {
        PackedColor(match self_ {
            Color::Transparent => Packed::from(Srgba::new(0, 0, 0, 0)),
            Color::Srgba(srgba) => Packed::from(srgba.into_format::<u8, u8>()),
            Color::Lcha(lcha) => {
                let srgba: Srgba = lcha.into_color();
                Packed::from(srgba.into_format::<u8, u8>())
            }
        })
    }
}

impl PackedColor {
    pub fn transparent() -> PackedColor {
        PackedColor(Packed::from(Srgba::new(0, 0, 0, 0)))
    }

    pub fn is_transparent(&self) -> bool {
        (self.0.color & 0xFF) == 0
    }

    pub fn is_opaque(&self) -> bool {
        (self.0.color & 0xFF) == 0xFF
    }

    pub fn stack(top: PackedColor, bottom: PackedColor) -> PackedColor {
        let top_alpha = top.0.color & 0xFF;
        let bottom_alpha = bottom.0.color & 0xFF;
        let alpha = (top_alpha * bottom_alpha) / 255;
        let color = (top.0.color & 0xFFFFFF00) | alpha;
        PackedColor(Packed::from(color))
    }
}

/// Specific kinds of colors
impl Color {
    pub fn red() -> Color {
        Color::Srgba(Srgba::new(1f32, 0f32, 0f32, 1f32))
    }

    pub fn green() -> Color {
        Color::Srgba(Srgba::new(0f32, 1f32, 0f32, 1f32))
    }

    pub fn blue() -> Color {
        Color::Srgba(Srgba::new(0f32, 0f32, 1f32, 1f32))
    }

    pub fn yellow() -> Color {
        Color::Srgba(Srgba::new(1f32, 1f32, 0f32, 1f32))
    }

    pub fn black() -> Color {
        Color::Srgba(Srgba::new(0f32, 0f32, 0f32, 1f32))
    }

    pub fn white() -> Color {
        Color::Srgba(Srgba::new(1f32, 1f32, 1f32, 1f32))
    }

    pub fn orange() -> Color {
        Color::Srgba(Srgba::new(1f32, 0.5f32, 0f32, 1f32))
    }
}

impl From<u32> for PackedColor {
    fn from(rgba: u32) -> Self {
        Self(Packed::from(rgba))
    }
}

impl Debug for PackedColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", Srgba::from(self.0))
    }
}

impl PartialEq for PackedColor {
    fn eq(&self, other: &Self) -> bool {
        self.0.color == other.0.color
    }

    fn ne(&self, other: &Self) -> bool {
        self.0.color != other.0.color
    }
}

impl Eq for PackedColor {}

// Not supported
/* #[cfg(feature = "tui")]
impl From<crossterm::style::Color> for PackedColor {
    fn from(color: crossterm::style::Color) -> PackedColor {
        PackedColor(match color {
            crossterm::style::Color::Reset => Srgba::new(0, 0, 0, 0).into(),
            crossterm::style::Color::Black => Srgba::new(0, 0, 0, 255).into(),
            crossterm::style::Color::DarkGrey => Srgba::new(128, 128, 128, 255).into(),
            crossterm::style::Color::Red => Srgba::new(255, 0, 0, 255).into(),
            crossterm::style::Color::DarkRed => Srgba::new(128, 0, 0, 255).into(),
            crossterm::style::Color::Green => Srgba::new(0, 255, 0, 255).into(),
            crossterm::style::Color::DarkGreen => Srgba::new(0, 128, 0, 255).into(),
            crossterm::style::Color::Yellow => Srgba::new(255, 255, 0, 255).into(),
            crossterm::style::Color::DarkYellow => Srgba::new(128, 128, 0, 255).into(),
            crossterm::style::Color::Blue => Srgba::new(0, 0, 255, 255).into(),
            crossterm::style::Color::DarkBlue => Srgba::new(0, 0, 128, 255).into(),
            crossterm::style::Color::Magenta => Srgba::new(255, 0, 255, 255).into(),
            crossterm::style::Color::DarkMagenta => Srgba::new(128, 0, 128, 255).into(),
            crossterm::style::Color::Cyan => Srgba::new(0, 255, 255, 255).into(),
            crossterm::style::Color::DarkCyan => Srgba::new(0, 128, 128, 255).into(),
            crossterm::style::Color::White => Srgba::new(255, 255, 255, 255).into(),
            crossterm::style::Color::Grey => Srgba::new(192, 192, 192, 255).into(),
            crossterm::style::Color::Rgb { r, g, b } => Srgba::new(r, g, b, 255).into(),
            crossterm::style::Color::AnsiValue(_) => Srgba::new(0, 0, 0, 255).into(),
        })
    }
} */

#[cfg(feature = "tui")]
impl From<PackedColor> for crossterm::style::Color {
    fn from(color: PackedColor) -> Self {
        crossterm::style::Color::Rgb {
            r: ((color.0.color & 0xFF000000) >> 24) as u8,
            g: ((color.0.color & 0x00FF0000) >> 16) as u8,
            b: ((color.0.color & 0x0000FF00) >> 8) as u8,
        }
    }
}