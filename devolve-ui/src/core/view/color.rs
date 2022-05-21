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
            Color::Transparent => Srgba::new(0, 0, 0, 0).into(),
            Color::Srgba(srgba) => srgba.into_format::<u8, u8>().into(),
            Color::Lcha(lcha) => {
                let srgba: Srgba = lcha.into_color();
                srgba.into_format::<u8, u8>().into()
            }
        })
    }
}

impl PackedColor {
    pub fn transparent() -> PackedColor {
        PackedColor(Srgba::new(0, 0, 0, 0).into())
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