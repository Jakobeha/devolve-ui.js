use palette::{Packed, Lcha, Srgba, IntoColor};
use palette::rgb::channels;

#[derive(Debug, Clone, Copy)]
pub enum Color {
    Transparent,
    Srgba(Srgba),
    Lcha(Lcha),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PackedColor(Packed<channels::Rgba>);

impl From<&Color> for PackedColor {
    fn from(self_: &Color) -> PackedColor {
        PackedColor(match self_ {
            Color::Transparent => Srgba::new(0, 0, 0, 0).into(),
            Color::Srgba(srgba) => srgba.into(),
            Color::Lcha(lcha) => {
                let srgba: Srgba = lcha.into_color();
                srgba.into()
            }
        })
    }
}

impl PackedColor {
    pub fn transparent() -> PackedColor {
        PackedColor(Srgba::new(0, 0, 0, 0).into())
    }

    pub fn is_transparent(&self) -> bool {
        (self.0 & 0xFF) == 0
    }

    pub fn is_opaque(&self) -> bool {
        (self.0 & 0xFF) == 0xFF
    }

    pub fn stack(top: PackedColor, bottom: PackedColor) -> PackedColor {
        let top_alpha = top.0 & 0xFF;
        let bottom_alpha = bottom.0 & 0xFF;
        let alpha = (top_alpha * bottom_alpha) / 255;
        let color = (top.0 & 0xFFFFFF00) | alpha;
        PackedColor(color)
    }
}