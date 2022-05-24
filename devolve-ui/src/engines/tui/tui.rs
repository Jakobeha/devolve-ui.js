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
use crate::core::renderer::renderer::VRender;
use crate::core::view::color::Color;
use crate::core::view::layout::err::LayoutError;
use crate::core::view::layout::geom::{BoundingBox, Rectangle, Size};
use crate::core::view::layout::parent_bounds::{DimsStore, ParentBounds};
use crate::core::view::view::VView;
use crate::view_data::tui::TuiViewData;
use crate::engines::tui::layer::RenderLayer;
use crate::view_data::attrs::{BorderStyle, DividerStyle, TextWrapMode};

#[cfg(target_family = "unix")]
lazy_static! {
    static ref SIGWINCH_CALLBACKS: RwLock<Vec<Box<dyn Fn() -> ()>>> = RwLock::new(Vec::new());
}

#[cfg(target_family = "unix")]
extern "C" fn sigwinch_handler_body(_: c_int) {
    if let Ok(callbacks) = (SIGWINCH_CALLBACKS as RwLock<Vec<Box<dyn FnMut(ParentBounds)>>>).read() {
        for callback in callbacks.iter() {
            callback();
        }
    }
}

#[cfg(target_family = "unix")]
const SIGWINCH_HANDLER: sighandler_t = sigwinch_handler_body as extern "C" fn(c_int) as *mut c_void as sighandler_t;

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

impl <Input: Read, Output: Write> TuiEngine<Input, Output> {
    pub fn new(config: TuiConfig<Input, Output>) -> Self {
        TuiEngine {
            config
        }
    }

    fn render_text(&self, bounds: &BoundingBox, color: &Color, wrap_mode: &TextWrapMode, lines: Lines<'_>) -> RenderLayer {
        todo!()
    }

    fn render_color(&self, bounds: &BoundingBox, color: &Color) -> RenderLayer {
        todo!()
    }

    fn render_border(&self, bounds: &BoundingBox, color: &Color, style: &BorderStyle) -> RenderLayer {
        todo!()
    }

    fn render_divider(&self, bounds: &BoundingBox, color: &Color, style: &DividerStyle) -> RenderLayer {
        todo!()
    }

    fn render_source(&self, bounds: &BoundingBox, source: &str) -> (RenderLayer, Size) {
        todo!()
    }
}

impl <Input: Read, Output: Write> RenderEngine for TuiEngine<Input, Output> {
    type ViewData = TuiViewData;
    type RenderLayer = RenderLayer;

    fn get_root_dimensions(&self) -> ParentBounds {
        let size = if let Some(size) = &self.config.override_size {
            size.clone()
        } else if let Some((width, height)) = terminal::size() {
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

    fn on_resize(&mut self, callback: Box<dyn Fn() -> ()>) {
        #[cfg(target_family = "unix")]
        unsafe {
            SIGWINCH_CALLBACKS.write().expect("coudln't add resize callback for some reason").push(callback);
            signal(SIGWINCH, SIGWINCH_HANDLER);
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
        RenderLayer::collapse(batch).write(&mut self.config.output);
    }

    fn clear(&mut self) {
        do_io(|| {
            // Clear scrollback
            write!(self.config.output, "\x1b[2J")?;
            Ok(())
        })
    }

    fn make_render(&self, bounds: &BoundingBox, column_size: &Size, view: &Box<VView<Self::ViewData>>, mut render: VRender<RenderLayer>) -> Result<VRender<RenderLayer>, LayoutError> {
        match &view.d {
            TuiViewData::Box {
                children: _children,
                sub_layout: _sub_layout,
                clip,
                extend
            } => {
                if clip || extend {
                    let rect = match bounds.as_rectangle() {
                        Ok(rect) => Some(&rect),
                        Err(layout_error) => {
                            error!("layout error getting rect to clip view {}: {}", view.id(), layout_error);
                            None
                        }
                    };
                    if clip && extend {
                        render.clip_and_extend(rect);
                    } else if clip {
                        render.clip(rect);
                    } else if extend {
                        render.extend(rect);
                    }
                }
            }
            TuiViewData::Text { text, color, wrap_mode } => {
                let lines = text.lines();
                let bounds = bounds.with_default_size(&Size {
                    width: lines.clone().map(|line| line.len()).max().unwrap_or(0) as f32,
                    height: lines.clone().len() as f32
                });
                let rect = bounds.as_rectangle().expect("didn't expect a layout error would be possible here after with_default_size");
                let layer = self.render_text(&bounds, color, wrap_mode, lines);
                render.insert(bounds.z, Some(&rect), layer);
            }
            TuiViewData::Color { color } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Fill-color requires explicit size"))?;
                let layer = self.render_color(bounds, color);
                render.insert(bounds.z, Some(&rect), layer);
            },
            TuiViewData::Border { color, style } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Border requires explicit size"))?;
                let layer = self.render_border(bounds, color, style);
                render.insert(bounds.z, Some(&rect), layer);
            },
            TuiViewData::Divider { color, style } => {
                let rect = bounds.as_rectangle().map_err(|err| err.add_description("Divider requires explicit size"))?;
                let layer = self.render_divider(bounds, color, style);
                render.insert(bounds.z, Some(&rect), layer);
            },
            TuiViewData::Source { source } => {
                let (layer, size) = self.render_image(bounds, column_size, source)?;
                let rect = bounds.as_rectangle_with_default_size(&size);
                render.insert(bounds.z, Some(&rect), layer);
            }
        }
        Ok(render)
    }
}