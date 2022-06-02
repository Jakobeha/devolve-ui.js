use std::cmp::{max, min};
use std::{env, ptr};
use std::io::Read;
use std::os::raw::{c_char, c_int, c_uchar, c_void};
use std::slice;
use semver::Version;
use png;
use sixel_sys;
use sixel_sys::status::{Status as SixelStatus};
use base64;
use crate::core::view::color::PackedColor;
use crate::engines::tui::layer::{RenderCell, RenderLayer};

pub enum ImageSupport {
    Fallback,
    Iterm,
    Kitty,
    Sixel
}

const SIXEL_TERMINALS: [&'static str; 13] = [
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

impl ImageSupport {
    pub fn get() -> Self {
        let terminal = env::var("LC_TERMINAL")
            .or_else(|| env::var("TERM_PROGRAM"))
            .unwrap_or_else(|| String::new());
        let terminal_version = env::var("LC_TERMINAL_VERSION")
            .or_else(|| env::var("TERM_PROGRAM_VERSION"))
            .ok()
            .and_then(|s| Version::parse(&s).ok());
        let konsole_version: usize = env::var("KONSOLE_VERSION")
            .unwrap_or_else(|| String::new())
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
        } else if SIXEL_TERMINALS.any(|prefix| terminal.start_with(prefix)) {
            Self::Sixel
        } else {
            Self::Fallback
        }
    }
}

pub struct Image<R: Read> {
    pub width: u16,
    pub height: u16,
    pub data: ImageData<R>
}

pub enum ImageData<R: Read> {
    RGBA8(Vec<u8>),
    PNG(R)
}

impl <R: Read> ImageData<R> {
    fn into_rgba8(self) -> Result<Vec<u8>, String> {
        Ok(match self {
            ImageData::RGBA8(data) => data,
            ImageData::PNG(input) => {
                let mut decoder = png::Decoder::new(input);
                let mut reader = decoder.read_info()?;
                let mut buf = vec![0; reader.output_buffer_size()];
                let _info = reader.next_frame(&mut buf)?;
                buf
            }
        })
    }

    fn into_png(self, width: u32, height: u32) -> Result<Vec<u8>, String> {
        Ok(match self {
            ImageData::RGB8(data) => {
                let mut result = vec![0; width as usize * height as usize * 4];
                let mut encoder = png::Encoder::new(&mut result, width, height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                let mut writer = encoder.write_header()?;
                writer.write_image_data(&data)?;
                result
            }
            ImageData::PNG(mut input) => {
                let mut result = Vec::new();
                input.read_to_end(&mut result)?;
                result
            }
        })
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

fn calculate_scaled_width_height(image_width: u16, image_height: u16, width: Measurement, height: Measurement, handle_aspect_ratio: HandleAspectRatio) -> Result<(u16, u16), String> {
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

fn calculate_scaled_width_height_pixels(image_width: u16, image_height: u16, width: u16, height: u16, handle_aspect_ratio: HandleAspectRatio) -> Result<(u16, u16), String> {
    match handle_aspect_ratio {
        _ if (width as f32 / image_width as f32 * image_height as f32) as u16 == height => Ok((width, height)),
        HandleAspectRatio::Complain => Err(format!("HandleAspectRatio::Complain is set but image aspect ratio ({}x{} = {}) does not match specified aspect ratio ({}x{} = {})", image_width, image_height, image_width as f32 / image_height as f32, width, height, width as f32 / height as f32)),
        HandleAspectRatio::Fit => {
            let fixed_width = min(width as f32, (height as f32 / image_height as f32) * image_width as f32) as u16;
            let fixed_height = min(height as f32, (width as f32 / image_width as f32) * image_height as f32) as u16;
            Ok((fixed_width, fixed_height))
        }
        HandleAspectRatio::Fill => {
            let fixed_width = max(width as f32, (height as f32 / image_height as f32) * image_width as f32) as u16;
            let fixed_height = max(height as f32, (width as f32 / image_width as f32) * image_height as f32) as u16;
            Ok((fixed_width, fixed_height))
        }
        HandleAspectRatio::Stretch => Ok((width, height))
    }
}

fn calculate_scaled_width_height_ratio(image_width: y16, image_height: u16, width_ratio: f32, height_ratio: f32, handle_aspect_ratio: HandleAspectRatio) -> Result<(u16, u16), String> {
    match handle_aspect_ratio {
        _ if width == height => Ok(((image_width as f32 * width_ratio) as u16, (image_height as f32 * height_ratio) as u16)),
        HandleAspectRatio::Complain => Err(format!("HandleAspectRatio::Complain is set but ratio is not 1:1)")),
        HandleAspectRatio::Fit => {
            let ratio = min(width_ratio, height_ratio);
            Ok(((image_width as f32 * ratio) as u16, (image_height as f32 * ratio) as u16))
        },
        HandleAspectRatio::Fill => {
            let ratio = max(width_ratio, height_ratio);
            Ok(((image_width as f32 * ratio) as u16, (image_height as f32 * ratio) as u16))
        },
        HandleAspectRatio::Stretch => Ok(((image_width as f32 * width_ratio) as u16, (image_height as f32 * height_ratio) as u16))
    }
}

impl <R: Read> Image<R> {
    pub fn render(self, width: Measurement, height: Measurement, handle_aspect_ratio: HandleAspectRatio) -> Result<(RenderLayer, (u16, u16)), String> {
        let (width, height) = calculate_scaled_width_height(self.width, self.height, width, height, handle_aspect_ratio)?;
        if width == 0 || height == 0 {
            return Ok((RenderLayer::new(), (0, 0)));
        }
        let render = match ImageSupport::get() {
            ImageSupport::Fallback => self.render_fallback(width, height),
            ImageSupport::Iterm => self.render_iterm(width, height),
            ImageSupport::Kitty => self.render_kitty(width, height),
            ImageSupport::Sixel => self.render_sixel(width, height)
        }?;
        Ok((render, (width, height)))
    }

    fn render_fallback(self, width: u16, height: u16) -> Result<RenderLayer, String> {
        let width = width as f32 / column_size.width;
        let height = height as f32 / column_size.height;
        let data = self.data.into_rgba8()?;
        let scale_width = self.width as f32 / width;
        let scale_height = self.height as f32 / height;

        let mut result = RenderLayer::of(RenderCell::transparent(), width as usize, height as usize);
        for y1 in 0..<(f32::round(height) as usize) {
            let y2 = f32::floor(y1 as f32 * scale_height) as u16;
            let y2p1 = f32::floor((y1 as f32 + 0.5f32) * scale_height) as u16;
            for x1 in 0..<(f32::round(width) as usize) {
                let x2 = f32::floor(x1 as f32 * scale_width) as u16;
                let offset_bg = y2 as usize * self.width as usize + x2 as usize;
                let rgba_bg = u32::from_be_bytes(data[offset_bg..offset_bg +4].try_into().unwrap());
                let color_bg = PackedColor::from(rgba_bg);
                let offset_fg = y2p1 as usize * self.width as usize + x2 as usize;
                let rgba_fg = u32::from_be_bytes(data[offset_fg..offset_fg +4].try_into().unwrap());
                let color_fg = PackedColor::from(rgba_fg);
                if !color_fg.is_transparent() || !color_fg.is_transparent() {
                    result[(x1, y1)] = RenderCell::simple_char('▄', color_fg, color_bg);
                }
            }
        }
        Ok(result)
    }

    fn render_iterm(self, width: u16, height: u16) -> Result<RenderLayer, String> {
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

    fn render_kitty(self, width: u16, height: u16) -> Result<RenderLayer, String> {
        let width = width as f32 / column_size.width;
        let height = height as f32 / column_size.height;
        let data = self.data.into_rgba()?;
        // Kitty proprietary format: https://sw.kovidgoyal.net/kitty/graphics-protocol/#png-data
        // Kitty also supports PNG, which one is faster / preferred?
        let escape_sequence = format!(
            "\x1b_Gf=32;s={},v={},c={},r={},t=d,{}\x1b\\"
            self.width,
            self.height,
            f32::round(width) as u16,
            f32::round(height) as u16,
            base64::encode(data)
        );
        Ok(RenderLayer::escape_sequence_and_filler(escape_sequence, width as usize, height as usize))
    }

    fn render_sixel(self, width: u16, height: u16) -> Result<RenderLayer, String> {
        let data = self.data.into_rgba8()?;
        let escape_sequence = _render_sixel(&data, width, height).map_err(|status| format!("libsixel error code: {}", status))?;
        Ok(RenderLayer::escape_sequence_and_filler(escape_sequence, width as usize, height as usize))
    }
}

fn _render_sixel(rgba8: &[u8], width: u16, height: u16) -> Result<String, SixelStatus> {
    unsafe {
        let mut output_str = String::new();
        let output = sixel_sys::sixel_output_create(Some(read_sixel), &mut output_str as *mut String as *mut c_void);
        let status = sixel_sys::sixel_encode(rgba8 as *mut c_uchar, width as c_int, height as c_int, 8, ptr::null_mut(), output);
        if status != SixelStatus::Ok {
            return Err(status)
        }
        sixel_sys::sixel_output_destroy(output);
        Ok(output_str)
    }
}

unsafe extern "C" fn read_sixel(data: *mut c_char, size: c_int, priv_: *mut c_void) -> c_int {
    let output_str = priv_ as *mut String as &mut String;
    let data = slice::from_raw_parts(data as *mut u8, size as usize);
    output_str.push_str(str::from_utf8(data).unwrap());
    size
}