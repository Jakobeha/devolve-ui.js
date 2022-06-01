use crossterm::terminal;
#[cfg(target_family = "unix")]
use std::sync::RwLock;
#[cfg(target_family = "unix")]
use lazy_static::lazy_static;
#[cfg(target_family = "unix")]
use libc::{ioctl, TIOCGWINSZ, winsize, signal, SIGWINCH, sighandler_t, c_int, c_void, SIGINT};
use std::io;
use std::io::{Stdin, Stdout, Read, Write, stdin, stdout};
#[cfg(target_family = "unix")]
use std::os::unix::io::{AsRawFd, RawFd};
use std::str::Lines;
use crate::core::renderer::engine::RenderEngine;
use crate::core::renderer::render::VRender;
use crate::core::view::color::{Color, PackedColor};
use crate::core::view::layout::err::LayoutError;
use crate::core::view::layout::geom::{BoundingBox, Rectangle, Size};
use crate::core::view::layout::parent_bounds::{DimsStore, ParentBounds};
use crate::core::view::view::VView;
use crate::view_data::tui::tui::TuiViewData;
use crate::engines::tui::layer::{RenderCell, RenderLayer};
use crate::view_data::attrs::{BorderStyle, DividerDirection, DividerStyle, TextWrapMode};

#[cfg(target_family = "unix")]
lazy_static! {
    static ref SIGWINCH_CALLBACKS: RwLock<Vec<Box<dyn Fn() + Send + Sync>>> = RwLock::new(Vec::new());
}

#[cfg(target_family = "unix")]
extern "C" fn sigwinch_handler_body(_: c_int) {
    if let Ok(callbacks) = (&SIGWINCH_CALLBACKS as &RwLock<Vec<Box<dyn Fn() + Send + Sync>>>).read() {
        for callback in callbacks.iter() {
            callback();
        }
    }
}

#[cfg(target_family = "unix")]
fn sigwinch_handler() -> sighandler_t {
    sigwinch_handler_body as extern "C" fn(c_int) as *mut c_void as sighandler_t
}

#[derive(Debug)]
pub struct TuiConfig<Input: Read, Output: Write> {
    pub input: Input,
    pub output: Output,
    #[cfg(target_family = "unix")]
    pub termios_fd: Option<RawFd>,
    pub raw_mode: bool,
    pub override_size: Option<Size>
}

#[derive(Debug)]
pub struct TuiEngine<Input: Read, Output: Write> {
    config: TuiConfig<Input, Output>
}

const DEFAULT_SIZE: Size = Size {
    width: 80f32,
    height: 24f32
};

const DEFAULT_COLUMN_SIZE: Size = Size {
    width: 6f32,
    height: 12f32
};

impl Default for TuiConfig<Stdin, Stdout> {
    fn default() -> Self {
        let input = stdin();
        let output = stdout();
        let fd = output.as_raw_fd();
        TuiConfig {
            input,
            output,
            termios_fd: Some(fd),
            raw_mode: true,
            override_size: None
        }
    }
}

impl Default for TuiEngine<Stdin, Stdout> {
    fn default() -> Self {
        TuiEngine::new(TuiConfig::default())
    }
}

fn do_io<R>(action: impl FnOnce() -> io::Result<R>) -> R {
    match action() {
        Ok(r) => r,
        Err(e) => panic!("io error: {}", e)
    }
}

// TODO: Remove when implementing render_ functions
#[allow(unused_variables)]
impl <Input: Read, Output: Write> TuiEngine<Input, Output> {
    pub fn new(config: TuiConfig<Input, Output>) -> Self {
        TuiEngine {
            config
        }
    }

    fn render_text(&self, bounds: &BoundingBox, color: &Option<Color>, wrap_mode: &TextWrapMode, lines: Lines<'_>) -> RenderLayer {
        let width = bounds.width.map_or(usize::MAX, |width| f32::round(width) as usize);
        let height = bounds.height.map_or(usize::MAX, |height| f32::round(height) as usize);

        let mut result = RenderLayer::with_capacity(height);
        let mut next_out_line = Vec::new();
        'outer: for line in lines {
            let mut next_word = Vec::new();
            for char in line.chars() {
                if *wrap_mode == TextWrapMode::Word && char.is_alphanumeric() {
                    // add to word
                    // width will never be 0
                    for cell in RenderCell::char(char) {
                        next_word.push(cell)
                    }
                } else {
                    if next_word.len() > 0 {
                        // wrap line if necessary and add word
                        if next_out_line.len() + next_word.len() > width {
                            // next_word.length > 0 implies wrap == TextWrapMode::Word
                            // so wrap line
                            if result.height() == height {
                                // no more room
                                break 'outer;
                            }
                            result.push(next_out_line);
                            next_out_line = Vec::new();
                        }

                        // add word
                        next_out_line.append(&mut next_word);
                    }

                    let char_width = char.width().unwrap_or(0);
                    if char_width == 0 {
                        // zero-width char, so we add it to the last character so it's outside on overlap
                        next_out_line[next_out_line.length - 1] += char
                    } else {
                        // wrap if necessary and add char
                        if next_out_line.len() + char_width > width {
                            match wrap_mode {
                                TextWrapMode::Word | TextWrapMode::Char => {
                                    if result.height() == height {
                                        // no more room
                                        break 'outer;
                                    }
                                    result.push(next_out_line);
                                    next_out_line = Vec::new();
                                }
                                TextWrapMode::Clip => {
                                    // This breaks out of the switch and continues the for loop, avoiding next_out_line.push(char)
                                    continue;
                                }
                                TextWrapMode::Undefined => {
                                    eprintln!("text extended past width but wrap is undefined")
                                }
                            }
                        }

                        // add char
                        for cell in RenderCell::char(char) {
                            next_out_line.push(cell);
                        }
                    }
                }
            }


            // wrap line if necessary and add word
            if next_out_line.len() + next_word.len() > width {
                // next_word.length > 0 implies wrap == TextWrapMode::Word
                // so wrap line
                if result.height() == height {
                    // no more room
                    break;
                }
                result.push(next_out_line);
                next_out_line = Vec::new();
            }

            // add word
            next_out_line.append(&mut next_word);

            // add line
            if result.height() == height {
                // no more room
                break
            }
            result.push(next_out_line);
            next_out_line = Vec::new();
        }

        if let Some(color) = color {
            result.set_fg(color);
        }
        result.translate1(bounds);

        result
    }

    fn render_color(&self, rect: &Rectangle, color: &Color) -> RenderLayer {
        let width = f32::round(rect.width()) as usize;
        let height = f32::round(rect.height()) as usize;
        let color = PackedColor::from(*color);
        if width == 0 || height == 0 {
            return RenderLayer::default();
        }

        let mut result = RenderLayer::of(width, height, RenderCell::simple_char(' ', PackedColor::transparent(), color))
        result.translate2(rect.left, rect.top);
        result
    }

    fn render_border(&self, rect: &Rectangle, color: &Option<Color>, style: &BorderStyle) -> RenderLayer {
        let width = f32::round(rect.width()) as usize;
        let height = f32::round(rect.height()) as usize;
        let color = color.map_or(PackedColor::transparent(), PackedColor::from);
        if width == 0 || height == 0 {
            return RenderLayer::default();
        }

        let border = style.ascii_border();
        let mut result = RenderLayer::of(width, height, RenderCell::transparent());

        result[(0, 0)] = RenderCell::simple_char(border.top_left, color, PackedColor::transparent());
        result[(width - 1, 0)] = RenderCell::simple_char(border.top_right, color, PackedColor::transparent());
        result[(0, height - 1)] = RenderCell::simple_char(border.bottom_left, color, PackedColor::transparent());
        result[(width - 1, height - 1)] = RenderCell::simple_char(border.bottom_right, color, PackedColor::transparent());
        for x in 1..<(width - 1) {
            result[(x, 0)] = RenderCell::simple_char(if let Some(top_alt) = border.top_alt.and(x % 2 == 1) {
                top_alt
            } else {
                border.top
            }, color, PackedColor::transparent());
            result[(x, height - 1)] = RenderCell::simple_char(if let Some(bottom_alt) = border.bottom_alt.and(x % 2 == 1) {
                bottom_alt
            } else {
                border.bottom
            }, color, PackedColor::transparent());
        }
        for y in 1..<(height - 1) {
            result[(0, y)] = RenderCell::simple_char(if let Some(left_alt) = border.left_alt.and(y % 2 == 1) {
                left_alt
            } else {
                border.left
            }, color, PackedColor::transparent());
            result[(width - 1, y)] = RenderCell::simple_char(if let Some(right_alt) = border.right_alt.and(y % 2 == 1) {
                right_alt
            } else {
                border.right
            }, color, PackedColor::transparent());
        }

        result.translate2(rect.left, rect.top);
        result
    }

    fn render_divider(&self, x: f32, y: f32, length: f32, thickness: f32, color: &Option<Color>, style: &DividerStyle, direction: &DividerDirection) -> RenderLayer {
        let length = f32::round(length) as usize;
        let thickness = f32::round(thickness) as usize;
        let color = color.map_or(PackedColor::transparent(), PackedColor::from);
        if length == 0 || thickness == 0 {
            return RenderLayer::default();
        }

        let divider = style.ascii_divider();
        let mut result = match direction {
            DividerDirection::Horizontal => {
                let mut result = RenderLayer::of(length, 1, RenderCell::simple_char(divider.horizontal, color, PackedColor::transparent()));
                if let Some(horizontal_alt) = divider.horizontal_alt {
                    for x in 1..<length {
                        if x % 2 == 1 {
                            result[(x, 0)] = RenderCell::simple_char(horizontal_alt, color, PackedColor::transparent());
                        }
                    }
                }
                result
            }
            DividerDirection::Vertical => {
                let mut result = RenderLayer::of(1, length, RenderCell::simple_char(divider.vertical, color, PackedColor::transparent()));
                if let Some(vertical_alt) = divider.vertical_alt {
                    for y in 1..<length {
                        if y % 2 == 1 {
                            result[(0, y)] = RenderCell::simple_char(vertical_alt, color, None);
                        }
                    }
                }
            }
        };
        result.translate2(x, y);
        result
    }

    fn render_source(&self, bounds: &BoundingBox, column_size: &Size, source: &str) -> Result<(RenderLayer, Size), LayoutError> {
        todo!()
    }
}

impl <Input: Read, Output: Write> RenderEngine for TuiEngine<Input, Output> {
    type ViewData = TuiViewData;
    type RenderLayer = RenderLayer;

    fn get_root_dimensions(&self) -> ParentBounds {
        let size = if let Some(size) = &self.config.override_size {
            size.clone()
        } else if let Ok((width, height)) = terminal::size() {
            Size { width: width as f32, height: height as f32 }
        } else {
            DEFAULT_SIZE
        };
        let mut column_size: Size = DEFAULT_COLUMN_SIZE;
        #[cfg(target_family = "unix")]
        if let Some(fd) = self.config.termios_fd {
            let mut winsize: winsize = unsafe { std::mem::zeroed() };
            let status = unsafe { ioctl(fd, TIOCGWINSZ, &mut winsize) };
            if status == 0 {
                column_size.width = winsize.ws_xpixel as f32 / size.width;
                column_size.height = winsize.ws_ypixel as f32 / size.height;
            }
        }
        // Maybe in the future, set some global values in store
        ParentBounds::typical_root(size, column_size, DimsStore::new())
    }

    fn on_resize(&mut self, callback: Box<dyn Fn() + Send + Sync>) {
        #[cfg(target_family = "unix")]
        unsafe {
            SIGWINCH_CALLBACKS.write().expect("coudln't add resize callback for some reason").push(callback);
            signal(SIGWINCH, sigwinch_handler());
        }
    }

    fn start_rendering(&mut self) {
        do_io(|| {
            // Enter TUI mode
            write!(self.config.output, "\x1b[?1049h")?;
            // Clear scrollback
            write!(self.config.output, "\x1b[2J")?;
            // Hide cursor
            write!(self.config.output, "\x1b[25l")?;
            Ok(())
        })
    }

    fn stop_rendering(&mut self) {
        do_io(|| {
            // Show cursor
            write!(self.config.output, "\x1b[25h")?;
            // Clear scrollback
            write!(self.config.output, "\x1b[2J")?;
            // Exit TUI mode
            write!(self.config.output, "\x1b[?1049l")?;
            Ok(())
        })
    }

    fn write_render(&mut self, batch: VRender<RenderLayer>) {
        do_io(|| RenderLayer::collapse(batch).write(&mut self.config.output))
    }

    fn clear(&mut self) {
        do_io(|| {
            // Clear scrollback
            write!(self.config.output, "\x1b[2J")?;
            Ok(())
        })
    }

    fn make_render(
        &self,
        bounds: &BoundingBox,
        column_size: &Size,
        view: &Box<VView<Self::ViewData>>,
        mut render: VRender<RenderLayer>
    ) -> Result<VRender<RenderLayer>, LayoutError> {
        match &view.d {
            TuiViewData::Box {
                children: _children,
                sub_layout: _sub_layout,
                clip,
                extend
            } => {
                if *clip || *extend {
                    let rect = match bounds.as_rectangle() {
                        Ok(rect) => Some(rect),
                        Err(layout_error) => {
                            eprintln!("layout error getting rect to clip view {}: {}", view.id(), layout_error);
                            None
                        }
                    };
                    if *clip && *extend {
                        render.clip_and_extend(rect.as_ref());
                    } else if *clip {
                        render.clip(rect.as_ref());
                    } else if *extend {
                        render.extend(rect.as_ref());
                    }
                }
            }
            TuiViewData::Text { text, color, wrap_mode } => {
                let lines = text.lines();
                let bounds = bounds.with_default_size(&Size {
                    width: lines.clone().map(|line| line.len()).max().unwrap_or(0) as f32,
                    height: lines.clone().count() as f32
                });
                let rect = bounds.as_rectangle().expect("didn't expect a layout error would be possible here after with_default_size");
                let layer = self.render_text(&bounds, color, wrap_mode, lines);
                render.insert(bounds.z, Some(&rect), layer);
            }
            TuiViewData::Color { color } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Fill-color requires explicit size"))?;
                let layer = self.render_color(rect, color);
                render.insert(bounds.z, Some(&rect), layer);
            },
            TuiViewData::Border { color, style } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Border requires explicit size"))?;
                let layer = self.render_border(rect, color, style);
                render.insert(bounds.z, Some(&rect), layer);
            },
            TuiViewData::Divider { color, direction, style } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Divider requires explicit size"))?;
                let (length, thickness) = match direction {
                    DividerDirection::Horizontal => (rect.width(), rect.height()),
                    DividerDirection::Vertical => (rect.height(), rect.width()),
                };
                if thickness > 1f32 {
                    return Err(LayoutError::new("divider with thickness > 1 not supported in CLI mode"));
                }
                let layer = self.render_divider(rect.x, rect.y, length, thickness, color, style, direction)?;
                render.insert(bounds.z, Some(&rect), layer);
            },
            TuiViewData::Source { source } => {
                let (layer, size) = self.render_source(bounds, column_size, source)?;
                let rect = bounds.as_rectangle_with_default_size(&size);
                render.insert(bounds.z, Some(&rect), layer);
            }
        }
        Ok(render)
    }
}