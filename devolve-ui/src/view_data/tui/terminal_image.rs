//! Data types for `devolve_ui::engines::tui::terminal_image`.

use std::{env, fmt};
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use png;
use semver::Version;

// region tui image format
/// How the image is rendered to the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum TuiImageFormat {
    /// Rendered as unicode shade characters. For terminals which don't support color or ANSI codes.
    FallbackGray,
    /// Rendered as character blocks with 2 colors. For terminals which don't support "real" images
    FallbackColor,
    /// Proprietary format used by iTerm: https://iterm2.com/documentation-images.html
    Iterm,
    /// Proprietary format used by Kitty: https://sw.kovidgoyal.net/kitty/graphics-protocol/#transferring-pixel-data
    Kitty,
    /// Sixel format: https://github.com/saitoha/libsixel
    Sixel
}

const SIXEL_TERMINALS: [&'static str; 14] = [
    "xterm",
    "contour",
    "mlterm",
    "mintty",
    "msys2",
    "dxterm",
    "kermit",
    "zste",
    "wrq",
    "rlogin",
    "yaft",
    "recterm",
    "seq2gif",
    "cancer"
];

impl TuiImageFormat {
    /// Infer the image format for this terminal from the current environment.
    /// This should work for most terminals.
    pub fn infer_from_env() -> Self {
        let terminal = env::var("LC_TERMINAL")
            .or_else(|_err| env::var("TERM_PROGRAM"))
            .unwrap_or_else(|_err| String::new());
        let terminal_version = env::var("LC_TERMINAL_VERSION")
            .or_else(|_err| env::var("TERM_PROGRAM_VERSION"))
            .ok()
            .and_then(|s| Version::parse(&s).ok());
        let konsole_version: usize = env::var("KONSOLE_VERSION")
            .unwrap_or_else(|_err| String::new())
            .parse()
            .unwrap_or(0);
        if terminal.starts_with("iterm") && terminal_version.is_some_and(|v| v.major >= 3) {
            Self::Iterm
        } else if terminal.starts_with("kitty") {
            Self::Kitty
        } else if konsole_version > 220000 || (terminal.starts_with("konsole") && terminal_version.is_some_and(|v| v.major >= 22)) {
            // Konsole doesn't seem to set LC_TERMINAL or LC_TERMINAL_VERSION,
            // however it does set KONSOLE_VERSION.
            Self::Sixel
        } else if SIXEL_TERMINALS.iter().any(|prefix| terminal.starts_with(prefix)) {
            Self::Sixel
        } else {
            Self::FallbackColor
        }
    }
}
// endregion

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceFormat {
    RawRgba32Image {
        size: (u16, u16)
    },
    Png,
}

pub enum Source {
    Path(PathBuf),
    Data {
        data: Box<dyn Fn() -> Box<dyn Read>>,
        format: SourceFormat,
        size: (u16, u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Measurement {
    Auto,
    Pixels(u16),
    Ratio(f32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandleAspectRatio {
    Complain,
    Fit,
    Fill,
    Stretch
}

impl Default for HandleAspectRatio {
    fn default() -> Self {
        HandleAspectRatio::Complain
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct InferredFileExtension(pub Option<String>);

#[derive(Debug)]
pub enum FailedToGetSize {
    UnsupportedFileExtension(InferredFileExtension),
    IoError(io::Error),
    PngDecodingError(png::DecodingError),
}

impl Source {
    pub fn try_get_size(&self) -> Result<(u16, u16), FailedToGetSize> {
        Ok(match self {
            Source::Path(path) => {
                let format = self.try_get_format()?;
                match format {
                    SourceFormat::RawRgba32Image { size } => size,
                    SourceFormat::Png => {
                        let file = File::open(path)?;
                        let decoder = png::Decoder::new(file);
                        let info = decoder.read_info()?;
                        let (width, height) = info.info().size();
                        (width as u16, height as u16)
                    }
                }
            },
            Source::Data { size: (width, height), .. } => (*width, *height)
        })
    }

    pub fn try_get_format(&self) -> Result<SourceFormat, InferredFileExtension> {
        match self {
            Source::Path(path) => {
                let extension = path.extension()
                    .and_then(|extension| extension.to_str());
                extension.and_then(SourceFormat::try_from_extension)
                    .ok_or_else(|| InferredFileExtension(extension.map(String::from)))
            },
            Source::Data { format, .. } => Ok(*format)
        }
    }

    pub fn try_get_reader(&self) -> io::Result<Box<dyn Read>> {
        match self {
            Source::Path(path) => {
                let file = File::open(path)?;
                Ok(Box::new(file))
            },
            Source::Data { data, .. } => Ok(data())
        }
    }
}

impl SourceFormat {
    fn try_from_extension(extension: &str) -> Option<SourceFormat> {
        match extension {
            "png" => Some(SourceFormat::Png),
            _ if extension.starts_with("rgba32_") => {
                let (width_str, height_str) = extension["rgba32_".len()..].split_once('x')?;
                let width: u16 = width_str.parse().ok()?;
                let height: u16 = height_str.parse().ok()?;
                Some(SourceFormat::RawRgba32Image { size: (width, height) })
            }
            _ => None
        }
    }
}

impl Debug for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Source::Path(path) => write!(f, "Source::Path({:?})", path),
            Source::Data { data: _data, format, size } => write!(f, "Source::Data(_:Box<dyn Read>, {:?}, {:?})", format, size)
        }
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Source::Path(path) => write!(f, "{}", path.display()),
            Source::Data { data: _data, format, size: (width, height) } => write!(f, "<{} data, {}x{}>", format, width, height)
        }
    }
}

impl Display for SourceFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SourceFormat::RawRgba32Image { size: _ } => write!(f, "RGBA32"),
            SourceFormat::Png => write!(f, "PNG")
        }
    }
}

impl From<InferredFileExtension> for FailedToGetSize {
    fn from(err: InferredFileExtension) -> Self {
        Self::UnsupportedFileExtension(err)
    }
}

impl From<io::Error> for FailedToGetSize {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<png::DecodingError> for FailedToGetSize {
    fn from(err: png::DecodingError) -> Self {
        Self::PngDecodingError(err)
    }
}