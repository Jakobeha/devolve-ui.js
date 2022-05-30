#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Single,
    Card,
    Double,
    Rounded,
    Dashed,
    Thick,
    Ascii,
    AsciiDashed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerStyle {
    Single,
    Double,
    Dashed,
    Thick,
    Ascii,
    AsciiDashed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextWrapMode {
    Undefined,
    Word,
    Char,
    Clip
}

impl Default for TextWrapMode {
    fn default() -> Self {
        TextWrapMode::Undefined
    }
}