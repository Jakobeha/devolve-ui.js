use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use png;

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