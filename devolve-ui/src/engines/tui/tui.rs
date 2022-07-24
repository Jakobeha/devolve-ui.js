//! This module provides the tui render engine, which converts tui views into layers and gets tui render info.
//! Basically everything except rendering the layers themselves.

use crossterm::terminal;
#[cfg(target_family = "unix")]
use libc::{ioctl, TIOCGWINSZ, winsize};
use std::io;
use std::io::{ErrorKind, Read, Stdin, stdin, Stdout, stdout, Write};
#[cfg(target_family = "unix")]
use std::os::unix::io::{AsRawFd, RawFd};
use std::str::Lines;
#[cfg(feature = "input")]
use std::time::Duration;
use unicode_width::UnicodeWidthChar;
use crate::core::renderer::engine::RenderEngine;
use crate::core::renderer::render::VRender;
use crate::core::renderer::traceback::RenderTraceback;
use crate::core::view::color::{Color, PackedColor};
use crate::core::view::layout::err::LayoutError;
use crate::core::view::layout::geom::{BoundingBox, Rectangle, Size};
use crate::core::view::layout::parent_bounds::{DimsStore, ParentBounds};
use crate::core::view::view::VView;
use crate::view_data::tui::tui::{TuiBoxAttrs, TuiViewData};
use crate::engines::tui::layer::{RenderCell, RenderLayer};
#[cfg(feature = "tui-images")]
use crate::engines::tui::terminal_image::{Image, ImageRender};
use crate::view_data::attrs::{BorderStyle, DividerDirection, DividerStyle, TextWrapMode};
#[cfg(feature = "tui-images")]
use crate::view_data::tui::terminal_image;
#[cfg(feature = "tui-images")]
use crate::view_data::tui::terminal_image::{HandleAspectRatio, Source, TuiImageFormat};
#[cfg(feature = "input")]
use crate::core::renderer::engine::InputListeners;
#[cfg(feature = "time")]
use crate::core::renderer::renderer::RendererViewForEngineInTick;
#[cfg(all(feature = "time", feature = "input"))]
use crate::core::renderer::input::{Event, KeyEvent, ResizeEvent};

/// Raw mode? Read from stdin or tty?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiInputMode {
    /// Raw mode + read using crossterm.
    /// This is the only mode where events like backspaces, arrow keys, etc. are read.
    /// This is the only mode which supports mouse and resize events as well.
    Raw,
    /// Read u8 chars from stdin. Also called "cbreak" or "rare" mode (https://en.wikipedia.org/wiki/Terminal_mode).
    /// This is not intended for reading large amounts of data, it's mainly intended for mocking.
    ReadAscii,
}

#[derive(Debug)]
pub struct TuiConfig<Input: Read, Output: Write> {
    pub input: Input,
    pub output: Output,
    #[cfg(target_family = "unix")]
    pub termios_fd: Option<RawFd>,
    pub input_mode: TuiInputMode,
    pub output_ansi_escapes: bool,
    #[cfg(feature = "tui-images")]
    pub image_format: TuiImageFormat
}

#[derive(Debug)]
pub struct TuiEngine<Input: Read, Output: Write> {
    config: TuiConfig<Input, Output>,
    is_listening_for_input: bool,
    inferred_column_size: Size
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
            #[cfg(target_family = "unix")]
            termios_fd: Some(fd),
            input_mode: TuiInputMode::Raw,
            output_ansi_escapes: true,
            #[cfg(feature = "tui-images")]
            image_format: TuiImageFormat::infer_from_env()
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

const ASCII_BUF_SIZE: usize = 8;

impl <Input: Read, Output: Write> TuiEngine<Input, Output> {
    pub fn new(config: TuiConfig<Input, Output>) -> Self {
        TuiEngine {
            config,
            is_listening_for_input: false,
            inferred_column_size: DEFAULT_COLUMN_SIZE
        }
    }

    fn render_text(&self, bounds: &BoundingBox, color: &Option<Color>, wrap_mode: &TextWrapMode, lines: Lines<'_>, traceback: &RenderTraceback<TuiViewData>) -> RenderLayer {
        let width = bounds.width.map_or(usize::MAX, |width| f32::round(width) as usize);
        let height = bounds.height.map_or(usize::MAX, |height| f32::round(height) as usize);

        let mut result_lines = Vec::with_capacity(height);
        'outer: for line in lines {
            let mut first_zero_width_chars = Vec::new();
            let add_char = |vec: &mut Vec<RenderCell>, char: char, first_zero_width_chars: &mut Vec<char>| {
                let mut first = true;
                for mut cell in RenderCell::char(char) {
                    if first {
                        first = false;
                        for first_zero_width_char in first_zero_width_chars.drain(..) {
                            cell.prepend_zw_char(first_zero_width_char);
                        }
                    }
                    vec.push(cell)
                }
            };

            let mut next_out_line = Vec::new();
            let mut next_word = Vec::new();
            for char in line.chars() {
                if *wrap_mode == TextWrapMode::Word && char.is_alphanumeric() {
                    // width will never be 0 so we don't need to check that
                    add_char(&mut next_word, char, &mut first_zero_width_chars);
                } else {
                    if next_word.len() > 0 {
                        // wrap line if necessary and add word
                        if next_out_line.len() + next_word.len() > width {
                            // next_word.length > 0 implies wrap == TextWrapMode::Word
                            // so wrap line
                            if result_lines.len() == height {
                                // no more room
                                break 'outer;
                            }
                            result_lines.push(next_out_line);
                            next_out_line = Vec::new();
                        }

                        // add word
                        next_out_line.append(&mut next_word);
                    }

                    let char_width = char.width().unwrap_or(0);
                    if char_width == 0 {
                        // zero-width char, so we want to add it to the previous char in case it's a terminal escape.
                        // Otherwise we handle the case very specially with first_zero_width_chars
                        if next_out_line.is_empty() {
                            first_zero_width_chars.insert(0, char);
                        } else {
                            let next_out_line_len = next_out_line.len();
                            next_out_line[next_out_line_len - 1].append_zw_char(char);
                        }
                    } else {
                        // wrap if necessary and add char
                        if next_out_line.len() + char_width > width {
                            match wrap_mode {
                                TextWrapMode::Word | TextWrapMode::Char => {
                                    if result_lines.len() == height {
                                        // no more room
                                        break 'outer;
                                    }
                                    result_lines.push(next_out_line);
                                    next_out_line = Vec::new();
                                }
                                TextWrapMode::Clip => {
                                    // This breaks out of the switch and continues the for loop, avoiding next_out_line.push(char)
                                    continue;
                                }
                                TextWrapMode::Undefined => {
                                    log::warn!("text extended past width but wrap is undefined\n{}", traceback);
                                }
                            }
                        }

                        add_char(&mut next_out_line, char, &mut first_zero_width_chars);
                    }
                }
            }


            // wrap line if necessary and add word
            if next_out_line.len() + next_word.len() > width {
                // next_word.length > 0 implies wrap == TextWrapMode::Word
                // so wrap line
                if result_lines.len() == height {
                    // no more room
                    break;
                }
                result_lines.push(next_out_line);
                next_out_line = Vec::new();
            }

            // add word
            next_out_line.append(&mut next_word);

            // add line
            if result_lines.len() == height {
                // no more room
                break
            }
            result_lines.push(next_out_line);
        }

        let mut result = RenderLayer::from(result_lines);

        if let Some(color) = color {
            result.set_fg(*color);
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

        let mut result = RenderLayer::of(RenderCell::simple_char(' ', PackedColor::transparent(), color), width, height);
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
        let mut result = RenderLayer::of(RenderCell::transparent(), width, height);

        result[(0, 0)] = RenderCell::simple_char(border.left_top, color, PackedColor::transparent());
        result[(width - 1, 0)] = RenderCell::simple_char(border.right_top, color, PackedColor::transparent());
        result[(0, height - 1)] = RenderCell::simple_char(border.left_bottom, color, PackedColor::transparent());
        result[(width - 1, height - 1)] = RenderCell::simple_char(border.right_bottom, color, PackedColor::transparent());
        for x in 1..(width - 1) {
            result[(x, 0)] = RenderCell::simple_char(if let Some(top_alt) = border.top_alt.filter(|_| x % 2 == 1) {
                top_alt
            } else {
                border.top
            }, color, PackedColor::transparent());
            result[(x, height - 1)] = RenderCell::simple_char(if let Some(bottom_alt) = border.bottom_alt.filter(|_| x % 2 == 1) {
                bottom_alt
            } else {
                border.bottom
            }, color, PackedColor::transparent());
        }
        for y in 1..(height - 1) {
            result[(0, y)] = RenderCell::simple_char(if let Some(left_alt) = border.left_alt.filter(|_| y % 2 == 1) {
                left_alt
            } else {
                border.left
            }, color, PackedColor::transparent());
            result[(width - 1, y)] = RenderCell::simple_char(if let Some(right_alt) = border.right_alt.filter(|_| y % 2 == 1) {
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
                let mut result = RenderLayer::of(RenderCell::simple_char(divider.horizontal, color, PackedColor::transparent()), length, 1);
                if let Some(horizontal_alt) = divider.horizontal_alt {
                    for x in 1..length {
                        if x % 2 == 1 {
                            result[(x, 0)] = RenderCell::simple_char(horizontal_alt, color, PackedColor::transparent());
                        }
                    }
                }
                result
            }
            DividerDirection::Vertical => {
                let mut result = RenderLayer::of(RenderCell::simple_char(divider.vertical, color, PackedColor::transparent()), 1, length);
                if let Some(vertical_alt) = divider.vertical_alt {
                    for y in 1..length {
                        if y % 2 == 1 {
                            result[(0, y)] = RenderCell::simple_char(vertical_alt, color, PackedColor::transparent());
                        }
                    }
                }
                result
            }
        };
        result.translate2(x, y);
        result
    }

    #[cfg(feature = "tui-images")]
    fn render_source(&self, bounds: &BoundingBox, column_size: &Size, source: &Source, handle_aspect_ratio: HandleAspectRatio) -> Result<(RenderLayer, Size), LayoutError> {
        let width = bounds.width.map_or(terminal_image::Measurement::Auto, |width| {
            terminal_image::Measurement::Pixels((width * column_size.width) as u16)
        });
        let height = bounds.height.map_or(terminal_image::Measurement::Auto, |height| {
            terminal_image::Measurement::Pixels((height * column_size.height) as u16)
        });
        let image = Image::try_from(source).map_err(|err| LayoutError::new(format!("failed to load source: {}", err)))?;
        let ImageRender { mut layer, size_in_pixels: (width_pixels, height_pixels) } = image.render(
            self.config.image_format,
            width,
            height,
            handle_aspect_ratio,
            column_size
        ).map_err(|msg| LayoutError::new(format!("failed to render source {}: {}", source, msg)))?;
        let width = width_pixels as f32 / column_size.width;
        let height = height_pixels as f32 / column_size.height;
        layer.translate1(bounds);
        Ok((layer, Size { width, height }))
    }
}

#[cfg(feature = "input")]
impl <Input: Read, Output: Write> TuiEngine<Input, Output> {
    fn start_listening_for_input(&mut self) {
        self.is_listening_for_input = true;
    }

    fn stop_listening_for_input(&mut self) {
        self.is_listening_for_input = false;
    }
}

#[cfg(all(feature = "time", feature = "input"))]
impl <Input: Read, Output: Write> TuiEngine<Input, Output> {
    fn poll_crossterm<Root: RenderEngine>(&mut self, engine: RendererViewForEngineInTick<'_, Root>) {
        // Crossterm event
        match crossterm::event::poll(Duration::from_secs(0)) {
            Err(error) => log::warn!("error polling for terminal input: {}", error),
            Ok(false) => {},
            Ok(true) => match crossterm::event::read() {
                Err(error) => log::warn!("error reading terminal input after (successfully) polling: {}", error),
                Ok(event) => self.process_event(engine, event.into())
            }
        }
    }

    fn poll_ascii<Root: RenderEngine>(&mut self, engine: RendererViewForEngineInTick<'_, Root>) {
        loop {
            let mut buf = [0u8; ASCII_BUF_SIZE];
            match self.config.input.read(&mut buf) {
                Ok(num_read) => {
                    for i in 0..num_read {
                        let char = buf[i] as char;
                        engine.send_key_event(&KeyEvent::from(char));
                    }
                    if num_read < ASCII_BUF_SIZE {
                        break
                    }
                }
                Err(io_err) if io_err.kind() == ErrorKind::WouldBlock => break,
                Err(io_err) => log::warn!("error reading ascii from input: {}", io_err)
            }
        }
    }

    fn process_event<Root: RenderEngine>(&mut self, engine: RendererViewForEngineInTick<'_, Root>, event: Event) {
        match event {
            Event::Key(key) => engine.send_key_event(&key),
            Event::Mouse(mouse) => engine.send_mouse_event(&mouse),
            Event::Resize(resize) => {
                match &resize {
                    ResizeEvent::Column(new_size) => self.inferred_column_size = new_size.clone(),
                    ResizeEvent::Window(_new_size) => {}
                }
                engine.send_resize_event(&resize);
            }
        }
    }
}

impl <Input: Read, Output: Write> RenderEngine for TuiEngine<Input, Output> {
    type ViewData = TuiViewData;
    type RenderLayer = RenderLayer;

    fn get_root_dimensions(&self) -> ParentBounds {
        let size = match terminal::size() {
            // This is wrong, idk why it happens
            Ok((0, 0)) => DEFAULT_SIZE,
            Ok((width, height)) => Size { width: width as f32, height: height as f32 },
            Err(_) => DEFAULT_SIZE
        };
        let mut column_size: Size = self.inferred_column_size;
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

    fn start_rendering(&mut self) {
        do_io(|| {
            if self.config.input_mode == TuiInputMode::Raw {
                // Enter raw mode (don't hard fail if it doesn't work)
                let result = terminal::enable_raw_mode();
                if let Err(error) = result {
                    log::warn!("failed to enter raw mode: {}", error);
                }
            }
            if self.config.output_ansi_escapes {
                // Enter TUI mode
                write!(self.config.output, "\x1b[?1049h")?;
                // Clear scrollback
                write!(self.config.output, "\x1b[2J")?;
                // Hide cursor
                write!(self.config.output, "\x1b[25l")?;
            }
            Ok(())
        })
    }

    fn stop_rendering(&mut self) {
        do_io(|| {
            if self.config.output_ansi_escapes {
                // Show cursor
                write!(self.config.output, "\x1b[25h")?;
                // Clear scrollback
                write!(self.config.output, "\x1b[2J")?;
                // Exit TUI mode
                write!(self.config.output, "\x1b[?1049l")?;
            }
            if self.config.input_mode == TuiInputMode::Raw {
                // Exit raw mode
                terminal::disable_raw_mode()?;
            }
            Ok(())
        })
    }

    fn write_render(&mut self, batch: VRender<RenderLayer>) {
        do_io(|| RenderLayer::collapse(batch).write::<Output>(&mut self.config.output, self.config.output_ansi_escapes))
    }

    fn clear(&mut self) {
        do_io(|| {
            if self.config.output_ansi_escapes {
                // Clear scrollback
                write!(self.config.output, "\x1b[2J")?;
            }
            Ok(())
        })
    }

    fn make_render(
        &self,
        bounds: &BoundingBox,
        #[allow(unused)] // Will be unused unless #[cfg(feature = "tui-images")] is enabled
        column_size: &Size,
        view: &Box<VView<Self::ViewData>>,
        mut render: VRender<RenderLayer>,
        traceback: &RenderTraceback<Self::ViewData>
    ) -> Result<VRender<RenderLayer>, LayoutError> {
        match &view.d {
            TuiViewData::Box {
                children: _children,
                attrs: TuiBoxAttrs {
                    sub_layout: _sub_layout,
                    clip,
                    extend
                }
            } => {
                if *clip || *extend {
                    let rect = match bounds.as_rectangle() {
                        Ok(rect) => Some(rect),
                        Err(layout_error) => {
                            log::warn!("layout error getting rect to clip view {}: {}", view.id(), layout_error);
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
                let layer = self.render_text(&bounds, color, wrap_mode, lines, traceback);
                render.insert(bounds.z, Some(&rect), layer);
            }
            TuiViewData::Color { color } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Fill-color requires explicit size"))?;
                let layer = self.render_color(&rect, color);
                render.insert(bounds.z, Some(&rect), layer);
            },
            TuiViewData::Border { color, style } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Border requires explicit size"))?;
                let layer = self.render_border(&rect, color, style);
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
                let layer = self.render_divider(rect.left, rect.top, length, thickness, color, style, direction);
                render.insert(bounds.z, Some(&rect), layer);
            },
            #[cfg(feature = "tui-images")]
            TuiViewData::Source { source, handle_aspect_ratio } => {
                let (layer, size) = self.render_source(bounds, column_size, source, *handle_aspect_ratio)?;
                let rect = bounds.as_rectangle_with_default_size(&size);
                render.insert(bounds.z, Some(&rect), layer);
            }
        }
        Ok(render)
    }

    #[cfg(feature = "time")]
    fn tick<Root: RenderEngine>(&mut self, engine: RendererViewForEngineInTick<'_, Root>) {
        #[cfg(feature = "input")]
        if self.is_listening_for_input {
            match self.config.input_mode {
                TuiInputMode::Raw => self.poll_crossterm(engine),
                TuiInputMode::ReadAscii => self.poll_ascii(engine),
            }
        }
    }

    #[cfg(feature = "input")]
    fn update_input_listeners(&mut self, listeners: InputListeners) {
        let is_listening_for_input = listeners != InputListeners::empty();
        if is_listening_for_input && !self.is_listening_for_input {
            self.start_listening_for_input()
        } else if !is_listening_for_input && self.is_listening_for_input {
            self.stop_listening_for_input()
        }
    }
}