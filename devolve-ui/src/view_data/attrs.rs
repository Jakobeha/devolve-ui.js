#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Single,
    Card,
    Double,
    Rounded,
    Dashed,
    Thick,
    Ascii,
    AsciiDashed,
    AsciiRounded,
    AsciiRoundedDashed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerDirection {
    Horizontal,
    Vertical
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

#[derive(Debug)]
pub struct AsciiBorderStyle {
    pub left_top: char,
    pub top: char,
    pub top_alt: Option<char>,
    pub right_top: char,
    pub right: char,
    pub right_alt: Option<char>,
    pub right_bottom: char,
    pub bottom: char,
    pub bottom_alt: Option<char>,
    pub left_bottom: char,
    pub left: char,
    pub left_alt: Option<char>
}

#[derive(Debug)]
pub struct AsciiDividerStyle {
    pub horizontal: char,
    pub vertical: char,
    pub horizontal_alt: Option<char>,
    pub vertical_alt: Option<char>
}

impl BorderStyle {
    //noinspection DuplicatedCode
    pub fn ascii_border(&self) -> AsciiBorderStyle {
        AsciiBorderStyle {
            left_top: match self {
                BorderStyle::Single => '┌',
                BorderStyle::Card => '╓',
                BorderStyle::Double => '╔',
                BorderStyle::Rounded => '╭',
                BorderStyle::Dashed => '┌',
                BorderStyle::Thick => '┏',
                BorderStyle::Ascii => '+',
                BorderStyle::AsciiDashed => '+',
                BorderStyle::AsciiRounded => '/',
                BorderStyle::AsciiRoundedDashed => '/'
            },
            top: match self {
                BorderStyle::Single => '─',
                BorderStyle::Card => '─',
                BorderStyle::Double => '═',
                BorderStyle::Rounded => '─',
                BorderStyle::Dashed => '┄',
                BorderStyle::Thick => '━',
                BorderStyle::Ascii => '-',
                BorderStyle::AsciiDashed => '-',
                BorderStyle::AsciiRounded => '|',
                BorderStyle::AsciiRoundedDashed => '|'
            },
            top_alt: match self {
                BorderStyle::Single => None,
                BorderStyle::Card => None,
                BorderStyle::Double => None,
                BorderStyle::Rounded => None,
                BorderStyle::Dashed => None,
                BorderStyle::Thick => None,
                BorderStyle::Ascii => None,
                BorderStyle::AsciiDashed => Some(' '),
                BorderStyle::AsciiRounded => None,
                BorderStyle::AsciiRoundedDashed => Some(' ')
            },
            right_top: match self {
                BorderStyle::Single => '┐',
                BorderStyle::Card => '╖',
                BorderStyle::Double => '╗',
                BorderStyle::Rounded => '╮',
                BorderStyle::Dashed => '┐',
                BorderStyle::Thick => '┓',
                BorderStyle::Ascii => '+',
                BorderStyle::AsciiDashed => '+',
                BorderStyle::AsciiRounded => '\\',
                BorderStyle::AsciiRoundedDashed => '\\'
            },
            right: match self {
                BorderStyle::Single => '│',
                BorderStyle::Card => '║',
                BorderStyle::Double => '║',
                BorderStyle::Rounded => '│',
                BorderStyle::Dashed => '┆',
                BorderStyle::Thick => '┃',
                BorderStyle::Ascii => '|',
                BorderStyle::AsciiDashed => '|',
                BorderStyle::AsciiRounded => '|',
                BorderStyle::AsciiRoundedDashed => '|'
            },
            right_alt: match self {
                BorderStyle::Single => None,
                BorderStyle::Card => None,
                BorderStyle::Double => None,
                BorderStyle::Rounded => None,
                BorderStyle::Dashed => None,
                BorderStyle::Thick => None,
                BorderStyle::Ascii => None,
                BorderStyle::AsciiDashed => Some(' '),
                BorderStyle::AsciiRounded => None,
                BorderStyle::AsciiRoundedDashed => Some(' ')
            },
            right_bottom: match self {
                BorderStyle::Single => '┘',
                BorderStyle::Card => '╜',
                BorderStyle::Double => '╝',
                BorderStyle::Rounded => '╯',
                BorderStyle::Dashed => '┘',
                BorderStyle::Thick => '┛',
                BorderStyle::Ascii => '+',
                BorderStyle::AsciiDashed => '+',
                BorderStyle::AsciiRounded => '/',
                BorderStyle::AsciiRoundedDashed => '/'
            },
            bottom: match self {
                BorderStyle::Single => '─',
                BorderStyle::Card => '─',
                BorderStyle::Double => '═',
                BorderStyle::Rounded => '─',
                BorderStyle::Dashed => '┄',
                BorderStyle::Thick => '━',
                BorderStyle::Ascii => '-',
                BorderStyle::AsciiDashed => '-',
                BorderStyle::AsciiRounded => '-',
                BorderStyle::AsciiRoundedDashed => '-'
            },
            bottom_alt: match self {
                BorderStyle::Single => None,
                BorderStyle::Card => None,
                BorderStyle::Double => None,
                BorderStyle::Rounded => None,
                BorderStyle::Dashed => None,
                BorderStyle::Thick => None,
                BorderStyle::Ascii => None,
                BorderStyle::AsciiDashed => Some(' '),
                BorderStyle::AsciiRounded => None,
                BorderStyle::AsciiRoundedDashed => Some(' ')
            },
            left_bottom: match self {
                BorderStyle::Single => '└',
                BorderStyle::Card => '╙',
                BorderStyle::Double => '╚',
                BorderStyle::Rounded => '╰',
                BorderStyle::Dashed => '└',
                BorderStyle::Thick => '┗',
                BorderStyle::Ascii => '+',
                BorderStyle::AsciiDashed => '+',
                BorderStyle::AsciiRounded => '\\',
                BorderStyle::AsciiRoundedDashed => '\\'
            },
            left: match self {
                BorderStyle::Single => '│',
                BorderStyle::Card => '│',
                BorderStyle::Double => '│',
                BorderStyle::Rounded => '│',
                BorderStyle::Dashed => '┆',
                BorderStyle::Thick => '┃',
                BorderStyle::Ascii => '|',
                BorderStyle::AsciiDashed => '|',
                BorderStyle::AsciiRounded => '|',
                BorderStyle::AsciiRoundedDashed => '|'
            },
            left_alt: match self {
                BorderStyle::Single => None,
                BorderStyle::Card => None,
                BorderStyle::Double => None,
                BorderStyle::Rounded => None,
                BorderStyle::Dashed => None,
                BorderStyle::Thick => None,
                BorderStyle::Ascii => None,
                BorderStyle::AsciiDashed => Some(' '),
                BorderStyle::AsciiRounded => None,
                BorderStyle::AsciiRoundedDashed => Some(' ')
            }
        }
    }
}

impl DividerStyle {
    pub(crate) fn ascii_divider(&self) -> AsciiDividerStyle {
        AsciiDividerStyle {
            horizontal: match self {
                DividerStyle::Single => '─',
                DividerStyle::Double => '═',
                DividerStyle::Dashed => '╌',
                DividerStyle::Thick => '━',
                DividerStyle::Ascii => '-',
                DividerStyle::AsciiDashed => '-'
            },
            horizontal_alt: match self {
                DividerStyle::Single => None,
                DividerStyle::Double => None,
                DividerStyle::Dashed => None,
                DividerStyle::Thick => None,
                DividerStyle::Ascii => None,
                DividerStyle::AsciiDashed => Some(' ')
            },
            vertical: match self {
                DividerStyle::Single => '│',
                DividerStyle::Double => '║',
                DividerStyle::Dashed => '│',
                DividerStyle::Thick => '┃',
                DividerStyle::Ascii => '|',
                DividerStyle::AsciiDashed => '|'
            },
            vertical_alt: match self {
                DividerStyle::Single => None,
                DividerStyle::Double => None,
                DividerStyle::Dashed => None,
                DividerStyle::Thick => None,
                DividerStyle::Ascii => None,
                DividerStyle::AsciiDashed => Some(' ')
            }
        }
    }
}

impl Default for TextWrapMode {
    fn default() -> Self {
        TextWrapMode::Undefined
    }
}