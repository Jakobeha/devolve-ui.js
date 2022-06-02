use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceFormat {
    RawRGBA32Image,
    PNG,
}

#[derive(Debug, Clone)]
pub enum Source {
    Path(PathBuf),
    Data(Box<dyn Reader>, SourceFormat)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InferredFileExtension(pub Option<String>);

impl Source {
    pub fn try_get_format(&self) -> Result<SourceFormat, InferredFileExtension> {
        match self {
            Source::Path(path) => {
                let extension = path.extension()
                    .and_then(|extension| extension.to_str());
                extension.and_then(SourceFormat::try_from_extension)
                    .ok_or_else(|| InferredFileExtension(extension.map(String::new)))
            },
            Source::Data(_, format) => Ok(*format)
        }
    }

    pub fn try_into_reader(self) -> io::Result<Box<dyn Reader>> {
        match self {
            Source::Path(path) => {
                let file = File::open(path);
                Ok(Box::new(file))
            },
            Source::Data(reader, _) => Ok(reader)
        }
    }
}

impl SourceFormat {
    fn try_from_extension(extension: &str) -> Option<SourceFormat> {
        match extension {
            "png" => Some(SourceFormat::PNG),
            "rgba32" => Some(SourceFormat::RawRGBA32Image),
            _ => None
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            SourceFormat::RawRGBA32Image => "rgba32",
            SourceFormat::PNG => "png"
        }
    }
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Source::Path(path) => write!(f, "{}", path.display()),
            Source::Data(_, format) => write!(f, "<{} data>", format)
        }
    }
}

impl Display for SourceFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SourceFormat::RawRGBA32Image => write!(f, "RGBA32"),
            SourceFormat::PNG => write!(f, "PNG")
        }
    }
}