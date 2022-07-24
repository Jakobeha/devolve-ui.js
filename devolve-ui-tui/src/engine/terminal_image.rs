//! Draw an image to the terminal.
//! The image can be given custom dimensions, auto (renders at its actual size as best as possible),
//! or a combination of the 2, and you can also choose if / how to preserve the aspect ratio.
//!
//! The image will be rendered using an ANSI escape code if the environment is set for one of the
//! hardcoded terminals. Otherwise it will render as colored blocks using the fallback rendererer.
//!
//! Supported terminals and the ANSI escape codes used:
//! - iterm: [proprietary](https://iterm2.com/documentation-images.html)
//! - kitty: [proprietary](https://sw.kovidgoyal.net/kitty/graphics-protocol/#transferring-pixel-data)
//! - xterm: [sixel](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-The-DECRQSSEL-Request-for-Selective-Erase-of-Line) (Ctrl-f "Sixel Graphics")
//! - contour: sixel
//! - mlterm: sixel
//! - mintty: sixel
//! - msys2: sixel
//! - dxterm: sixel
//! - kermit: sixel
//! - zste: sixel
//! - wrq: sixel
//! - rlogin: sixel
//! - yaft: sixel
//! - recterm: sixel
//! - seq2gif: sixel
//! - cancer: sixel
//! - all others: fallback

use std::fs::File;
use std::io;
use std::io::Read;
use std::path::PathBuf;
use png;
use base64;
use sixel;
use sixel::status::{Error as SixelError};
use sixel_sys;
use derive_more::{Display, Error, From};
use crate::view::color::PackedColor;
use crate::view::layout::geom::Size;
use crate::engines::tui::layer::{RenderCell, RenderLayer};
use crate::view_data::tui::terminal_image::{FailedToGetSize, HandleAspectRatio, InferredFileExtension, Measurement, Source, SourceFormat, TuiImageFormat};

pub struct Image<R: Read> {
    pub width: u16,
    pub height: u16,
    pub data: ImageData<R>
}

pub enum ImageData<R: Read> {
    Rgba32(Vec<u8>),
    Png(R)
}

impl <R: Read> ImageData<R> {
    fn into_rgba32(self) -> Result<Vec<u8>, RenderError> {
        Ok(match self {
            ImageData::Rgba32(data) => data,
            ImageData::Png(input) => {
                let decoder = png::Decoder::new(input);
                let mut reader = decoder.read_info()?;
                let mut buf = vec![0; reader.output_buffer_size()];
                let _info = reader.next_frame(&mut buf)?;
                buf
            }
        })
    }

    fn into_png(self, width: u32, height: u32) -> Result<Vec<u8>, RenderError> {
        Ok(match self {
            ImageData::Rgba32(data) => {
                let mut result = vec![0; width as usize * height as usize * 4];
                {
                    let mut encoder = png::Encoder::new(&mut result, width, height);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    let mut writer = encoder.write_header()?;
                    writer.write_image_data(&data)?;
                }
                result
            }
            ImageData::Png(mut input) => {
                let mut result = Vec::new();
                input.read_to_end(&mut result)?;
                result
            }
        })
    }
}

#[derive(Debug, Display, Error, From)]
pub enum Source2ImageDataError {
    #[display(fmt = "{}", _0)]
    FailedToGetSize(FailedToGetSize),
    #[from(ignore)]
    #[display(fmt = "{}", "_0.0.as_ref().map_or(std::borrow::Cow::from(\"unsupported format\"), |e| std::borrow::Cow::from(format!(\"unsupported format: {}\", e)))")]
    UnsupportedFormat(#[error(not(source))] InferredFileExtension),
    #[display(fmt = "io error: {}", _0)]
    IoError(io::Error),
    #[display(fmt = "png decoding error: {}", _0)]
    PngDecodingError(png::DecodingError)
}

#[derive(Debug, Display, Error, From)]
pub enum RenderError {
    #[from(ignore)]
    #[display(fmt = "HandleAspectRatio::Complain is set, but {}", _0)]
    AspectRatioComplain(#[error(not(source))] String),
    #[display(fmt = "io error: {}", _0)]
    IoError(io::Error),
    #[display(fmt = "png encoding error: {}", _0)]
    PngEncodingError(png::EncodingError),
    #[display(fmt = "png decoding error: {}", _0)]
    PngDecodingError(png::DecodingError),
    #[display(fmt = "libsixel error: {:?}", _0)]
    SixelError(#[error(not(source))] SixelError)
}

impl TryFrom<&Source> for Image<Box<dyn Read>> {
    type Error = Source2ImageDataError;

    fn try_from(value: &Source) -> Result<Self, Self::Error> {
        let (width, height) = value.try_get_size()?;
        let data = match value.try_get_format() {
            Err(extension) => Err(Source2ImageDataError::UnsupportedFormat(extension))?,
            Ok(SourceFormat::RawRgba32Image { size: _size }) => {
                let mut data = Vec::new();
                value.try_get_reader()?.read_to_end(&mut data)?;
                ImageData::Rgba32(data)
            },
            Ok(SourceFormat::Png) => ImageData::Png(value.try_get_reader()?),
        };
        Ok(Image { data, width, height })
    }
}

fn calculate_scaled_width_height(image_width: u16, image_height: u16, width: Measurement, height: Measurement, handle_aspect_ratio: HandleAspectRatio) -> Result<(u16, u16), RenderError> {
    match (width, height) {
        (Measurement::Auto, Measurement::Auto) => Ok((image_width, image_height)),
        (Measurement::Pixels(width), Measurement::Auto) => {
            let height = ((width as f32 / image_width as f32) * image_height as f32) as u16;
            Ok((width, height))
        }
        (Measurement::Auto, Measurement::Pixels(height)) => {
            let width = ((height as f32 / image_height as f32) * image_width as f32) as u16;
            Ok((width, height))
        }
        (Measurement::Ratio(ratio), Measurement::Auto) |
        (Measurement::Auto, Measurement::Ratio(ratio)) => {
            Ok(((image_width as f32 * ratio) as u16, (image_height as f32 * ratio) as u16))
        }
        (Measurement::Pixels(width), Measurement::Pixels(height)) => {
            calculate_scaled_width_height_pixels(image_width, image_height, width, height, handle_aspect_ratio)
        }
        (Measurement::Ratio(width_ratio), Measurement::Ratio(height_ratio)) => {
            calculate_scaled_width_height_ratio(image_width, image_height, width_ratio, height_ratio, handle_aspect_ratio)
        }
        (Measurement::Pixels(width), Measurement::Ratio(height_ratio)) => {
            let height = (image_height as f32 * height_ratio) as u16;
            calculate_scaled_width_height_pixels(image_width, image_height, width, height, handle_aspect_ratio)
        },
        (Measurement::Ratio(width_ratio), Measurement::Pixels(height)) => {
            let width = (image_width as f32 * width_ratio) as u16;
            calculate_scaled_width_height_pixels(image_width, image_height, width, height, handle_aspect_ratio)
        }
    }
}

fn calculate_scaled_width_height_pixels(image_width: u16, image_height: u16, width: u16, height: u16, handle_aspect_ratio: HandleAspectRatio) -> Result<(u16, u16), RenderError> {
    match handle_aspect_ratio {
        _ if (width as f32 / image_width as f32 * image_height as f32) as u16 == height => Ok((width, height)),
        HandleAspectRatio::Complain => Err(RenderError::AspectRatioComplain(format!("image aspect ratio ({}x{} = {}) does not match specified aspect ratio ({}x{} = {})", image_width, image_height, image_width as f32 / image_height as f32, width, height, width as f32 / height as f32))),
        HandleAspectRatio::Fit => {
            let fixed_width = f32::min(width as f32, (height as f32 / image_height as f32) * image_width as f32) as u16;
            let fixed_height = f32::min(height as f32, (width as f32 / image_width as f32) * image_height as f32) as u16;
            Ok((fixed_width, fixed_height))
        }
        HandleAspectRatio::Fill => {
            let fixed_width = f32::max(width as f32, (height as f32 / image_height as f32) * image_width as f32) as u16;
            let fixed_height = f32::max(height as f32, (width as f32 / image_width as f32) * image_height as f32) as u16;
            Ok((fixed_width, fixed_height))
        }
        HandleAspectRatio::Stretch => Ok((width, height))
    }
}

fn calculate_scaled_width_height_ratio(image_width: u16, image_height: u16, width_ratio: f32, height_ratio: f32, handle_aspect_ratio: HandleAspectRatio) -> Result<(u16, u16), RenderError> {
    match handle_aspect_ratio {
        _ if width_ratio == height_ratio => Ok(((image_width as f32 * width_ratio) as u16, (image_height as f32 * height_ratio) as u16)),
        HandleAspectRatio::Complain => Err(RenderError::AspectRatioComplain(format!("ratio is not 1:1)"))),
        HandleAspectRatio::Fit => {
            let ratio = f32::min(width_ratio, height_ratio);
            Ok(((image_width as f32 * ratio) as u16, (image_height as f32 * ratio) as u16))
        },
        HandleAspectRatio::Fill => {
            let ratio = f32::max(width_ratio, height_ratio);
            Ok(((image_width as f32 * ratio) as u16, (image_height as f32 * ratio) as u16))
        },
        HandleAspectRatio::Stretch => Ok(((image_width as f32 * width_ratio) as u16, (image_height as f32 * height_ratio) as u16))
    }
}

pub struct ImageRender {
    pub layer: RenderLayer,
    pub size_in_pixels: (u16, u16)
}

impl ImageRender {
    pub fn empty() -> Self {
        Self {
            layer: RenderLayer::new(),
            size_in_pixels: (0, 0)
        }
    }
}

impl <R: Read> Image<R> {
    pub fn render(
        self,
        image_format: TuiImageFormat,
        width: Measurement,
        height: Measurement,
        handle_aspect_ratio: HandleAspectRatio,
        column_size: &Size,
    ) -> Result<ImageRender, RenderError> {
        let (width, height) = calculate_scaled_width_height(self.width, self.height, width, height, handle_aspect_ratio)?;
        if width == 0 || height == 0 {
            return Ok(ImageRender::empty());
        }
        let render = match image_format {
            TuiImageFormat::FallbackGray => self.render_fallback_gray(width, height, column_size),
            TuiImageFormat::FallbackColor => self.render_fallback_color(width, height, column_size),
            TuiImageFormat::Iterm => self.render_iterm(width, height),
            TuiImageFormat::Kitty => self.render_kitty(width, height, column_size),
            TuiImageFormat::Sixel => self.render_sixel(width, height)
        }?;
        Ok(ImageRender {
            layer: render,
            size_in_pixels: (width, height)
        })
    }

    fn render_fallback_gray(self, width: u16, height: u16, column_size: &Size) -> Result<RenderLayer, RenderError> {
        let width = f32::round(width as f32 / column_size.width) as usize;
        let height = f32::round(height as f32 / column_size.height) as usize;
        let data = self.data.into_rgba32()?;
        let scale_width = self.width as f32 / width as f32;
        let scale_height = self.height as f32 / height as f32;

        let mut result = RenderLayer::of(RenderCell::transparent(), width, height);
        for y1 in 0..height {
            let y2 = f32::floor(y1 as f32 * scale_height) as u16;
            for x1 in 0..width {
                let x2 = f32::floor(x1 as f32 * scale_width) as u16;
                let offset_bg = (y2 as usize * self.width as usize + x2 as usize) * 4;
                let rgba_bg = u32::from_be_bytes(data[offset_bg..offset_bg+4].try_into().unwrap());
                let color_bg = PackedColor::from(rgba_bg);
                if !color_bg.is_transparent() {
                    let grayscale_bg = color_bg.white();
                    let block = if grayscale_bg > 224 {
                        '█'
                    } else if grayscale_bg > 160 {
                        '▓'
                    } else if grayscale_bg > 96 {
                        '▒'
                    } else if grayscale_bg > 32 {
                        '░'
                    } else {
                        ' '
                    };
                    result[(x1, y1)] = RenderCell::simple_char(block, PackedColor::transparent(), PackedColor::transparent());
                }
            }
        }
        Ok(result)
    }

    fn render_fallback_color(self, width: u16, height: u16, column_size: &Size) -> Result<RenderLayer, RenderError> {
        let width = f32::round(width as f32 / column_size.width) as usize;
        let height = f32::round(height as f32 / column_size.height) as usize;
        let data = self.data.into_rgba32()?;
        let scale_width = self.width as f32 / width as f32;
        let scale_height = self.height as f32 / height as f32;

        let mut result = RenderLayer::of(RenderCell::transparent(), width, height);
        for y1 in 0..height {
            let y2 = f32::floor(y1 as f32 * scale_height) as u16;
            let y2p1 = f32::floor((y1 as f32 + 0.5f32) * scale_height) as u16;
            for x1 in 0..width {
                let x2 = f32::floor(x1 as f32 * scale_width) as u16;
                let offset_bg = (y2 as usize * self.width as usize + x2 as usize) * 4;
                let rgba_bg = u32::from_be_bytes(data[offset_bg..offset_bg+4].try_into().unwrap());
                let color_bg = PackedColor::from(rgba_bg);
                let offset_fg = (y2p1 as usize * self.width as usize + x2 as usize) * 4;
                let rgba_fg = u32::from_be_bytes(data[offset_fg..offset_fg+4].try_into().unwrap());
                let color_fg = PackedColor::from(rgba_fg);
                if !color_bg.is_transparent() || !color_fg.is_transparent() {
                    result[(x1, y1)] = RenderCell::simple_char('▄', color_fg, color_bg);
                }
            }
        }
        Ok(result)
    }

    fn render_iterm(self, width: u16, height: u16) -> Result<RenderLayer, RenderError> {
        let data = self.data.into_png(self.width as u32, self.height as u32)?;
        // iTerm proprietary format: https://iterm2.com/documentation-images.html
        // iTerm also supports sixel, which one is faster / preferred?
        let escape_sequence = format!(
            "\x1b]1337;File=inline=1;name={};width={}px;height={}px;preserveAspectRatio=0:{}\x1b\\",
            base64::encode("devolve-ui image"),
            width,
            height,
            base64::encode(data)
        );
        Ok(RenderLayer::escape_sequence_and_filler(escape_sequence, width as usize, height as usize))
    }

    pub fn render_kitty(self, width: u16, height: u16, column_size: &Size) -> Result<RenderLayer, RenderError> {
        let width = width as f32 / column_size.width;
        let height = height as f32 / column_size.height;
        let data = self.data.into_rgba32()?;
        // Kitty proprietary format: https://sw.kovidgoyal.net/kitty/graphics-protocol/#png-data
        // Kitty also supports Png, which one is faster / preferred?
        let escape_sequence = format!(
            "\x1b_Gf=32;s={},v={},c={},r={},t=d,{}\x1b\\",
            self.width,
            self.height,
            f32::round(width) as u16,
            f32::round(height) as u16,
            base64::encode(data)
        );
        Ok(RenderLayer::escape_sequence_and_filler(escape_sequence, width as usize, height as usize))
    }

    fn render_sixel(self, width: u16, height: u16) -> Result<RenderLayer, RenderError> {
        let data = self.data.into_rgba32()?;
        let escape_sequence = _render_sixel(&data, self.width, self.height, width, height)?;
        Ok(RenderLayer::escape_sequence_and_filler(escape_sequence, width as usize, height as usize))
    }
}

fn _render_sixel(rgba32: &[u8], image_width: u16, image_height: u16, width: u16, height: u16) -> Result<String, RenderError> {
    let frame = sixel::encoder::QuickFrameBuilder::new()
        .width(image_width as usize)
        .height(image_height as usize)
        .format(sixel_sys::PixelFormat::RGBA8888)
        .pixels(Vec::from(rgba32));

    let encoder = sixel::encoder::Encoder::new()?;
    encoder.set_width(sixel::optflags::SizeSpecification::Pixel(width as u64))?;
    encoder.set_height(sixel::optflags::SizeSpecification::Pixel(height as u64))?;

    let output_dir = tempfile::tempdir()?;
    let mut output_path = PathBuf::from(output_dir.path());
    output_path.push("sixel.out");
    encoder.set_output(&output_path)?;

    encoder.encode_bytes(frame)?;

    let mut output_file = File::options().read(true).open(output_path)?;
    let mut output = String::new();
    output_file.read_to_string(&mut output)?;

    Ok(output)
}