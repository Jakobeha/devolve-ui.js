//! This module defines tui render layers and provides the code to render them.

// Thanks to http://xn--rpa.cc/irl/term.html for explaining obscure terminal escape codes and behaviors
use std::io;
use std::iter;
use std::mem;
use std::fmt::Write;
use std::ops::{Index, IndexMut};
use crossterm::{Command, cursor};
use crossterm::style;
use unicode_width::UnicodeWidthChar;
use replace_with::replace_with_or_abort;
use crate::core::misc::io_write_2_fmt_write::IoWrite2FmtWrite;
use crate::core::view::color::PackedColor;
use crate::core::view::layout::geom::{BoundingBox, Rectangle};
use crate::core::renderer::render::{VRender, VRenderLayer};

#[derive(Debug, Clone)]
pub enum RenderCellContent {
    TransparentChar,
    Char(char),
    // e.g. image
    ManyChars(String),
    // e.g. padding where image would be
    ZeroChars,
    TransparentCharWithZeroWidths {
        prefix: String,
        suffix: String
    }
}

#[derive(Debug, Clone)]
pub struct RenderCell {
    pub content: RenderCellContent,
    pub fg: PackedColor,
    pub bg: PackedColor
}

#[derive(Debug, Clone)]
pub struct RenderLayer(Vec<Vec<RenderCell>>);

impl RenderCell {
    pub fn new(content: RenderCellContent) -> Self {
        RenderCell {
            content,
            fg: PackedColor::transparent(),
            bg: PackedColor::transparent()
        }
    }

    pub fn transparent() -> Self {
        Self::new(RenderCellContent::TransparentChar)
    }

    pub fn simple_char(char: char, fg: PackedColor, bg: PackedColor) -> Self {
        assert_eq!(char.width(), Some(1), "char width is not 1, use RenderCell::char() instead");
        RenderCell {
            content: RenderCellContent::Char(char),
            fg: fg,
            bg: bg
        }
    }

    pub fn char(char: char) -> impl Iterator<Item = Self> {
        let char_width = char.width().unwrap_or(0);
        assert_ne!(char_width, 0, "char width is 0, handle explicitly");
        iter::once(Self::new(RenderCellContent::Char(char))).chain(
            (1..char_width).map(|_i| Self::transparent())
        )
    }

    pub fn many_chars(chars: String) -> Self {
        Self::new(RenderCellContent::ManyChars(chars))
    }

    pub fn zero_width() -> Self {
        Self::new(RenderCellContent::ZeroChars)
    }

    pub fn prepend_zw_char(&mut self, prefix_char: char) {
        replace_with_or_abort(&mut self.content, |content| match content {
            RenderCellContent::TransparentChar => RenderCellContent::TransparentCharWithZeroWidths {
                prefix: prefix_char.to_string(),
                suffix: String::new()
            },
            RenderCellContent::Char(char) => RenderCellContent::ManyChars(format!("{}{}", prefix_char, char)),
            RenderCellContent::ManyChars(str) => RenderCellContent::ManyChars(format!("{}{}", prefix_char, str)),
            RenderCellContent::ZeroChars => RenderCellContent::Char(prefix_char),
            RenderCellContent::TransparentCharWithZeroWidths { prefix, suffix } => RenderCellContent::TransparentCharWithZeroWidths {
                prefix: format!("{}{}", prefix_char, prefix),
                suffix
            }
        })
    }

    pub fn append_zw_char(&mut self, suffix_char: char) {
        replace_with_or_abort(&mut self.content, |content| match content {
            RenderCellContent::TransparentChar => RenderCellContent::TransparentCharWithZeroWidths {
                prefix: String::new(),
                suffix: suffix_char.to_string()
            },
            RenderCellContent::Char(char) => RenderCellContent::ManyChars(format!("{}{}", char, suffix_char)),
            RenderCellContent::ManyChars(str) => RenderCellContent::ManyChars(format!("{}{}", str, suffix_char)),
            RenderCellContent::ZeroChars => RenderCellContent::Char(suffix_char),
            RenderCellContent::TransparentCharWithZeroWidths { prefix, suffix } => RenderCellContent::TransparentCharWithZeroWidths {
                prefix,
                suffix: format!("{}{}", suffix, suffix_char)
            }
        })
    }

    pub fn add_zw_prefix_suffix(&mut self, prefix: String, suffix: String) {
        if !prefix.is_empty() || !suffix.is_empty() {
            replace_with_or_abort(&mut self.content, |content| match content {
                RenderCellContent::TransparentChar => RenderCellContent::TransparentCharWithZeroWidths {
                    prefix,
                    suffix
                },
                RenderCellContent::Char(char) => RenderCellContent::ManyChars(format!("{}{}{}", prefix, char, suffix)),
                RenderCellContent::ManyChars(str) => RenderCellContent::ManyChars(format!("{}{}{}", prefix, str, suffix)),
                RenderCellContent::ZeroChars if prefix.len() == 1 && suffix.is_empty() => RenderCellContent::Char(prefix.chars().next().unwrap()),
                RenderCellContent::ZeroChars if prefix.is_empty() && suffix.len() == 1 => RenderCellContent::Char(suffix.chars().next().unwrap()),
                RenderCellContent::ZeroChars => RenderCellContent::ManyChars(format!("{}{}", prefix, suffix)),
                RenderCellContent::TransparentCharWithZeroWidths { prefix: prefix2, suffix: suffix2 } => RenderCellContent::TransparentCharWithZeroWidths {
                    prefix: format!("{}{}", prefix, prefix2),
                    suffix: format!("{}{}", suffix2, suffix)
                }
            })
        }
    }
}

impl RenderLayer {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(height: usize) -> Self {
        Self(Vec::with_capacity(height))
    }

    pub fn from_iter(lines: impl Iterator<Item = impl Iterator<Item = RenderCell>>) -> Self {
        Self(lines.map(|line| line.collect()).collect())
    }

    pub fn of(cell: RenderCell, width: usize, height: usize) -> Self {
        Self(vec![vec![cell; width]; height])
    }

    pub fn escape_sequence_and_filler(escape_sequence: String, width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0, "width and height must be non-zero");
        let mut layer = Self::of(RenderCell::zero_width(), width, height);
        layer[(0, 0)].content = RenderCellContent::ManyChars(escape_sequence);
        layer
    }

    pub fn set_fg(&mut self, color: impl Into<PackedColor>) {
        let color = color.into();
        for line in self.0.iter_mut() {
            for col in line.iter_mut() {
                col.fg = color
            }
        }
    }

    pub fn set_bg(&mut self, color: impl Into<PackedColor>) {
        let color = color.into();
        for line in self.0.iter_mut() {
            for mut cell in line.iter_mut() {
                cell.bg = color
            }
        }
    }

    pub fn translate1(&mut self, bounds: &BoundingBox) {
        let width = bounds.width.unwrap_or_else(|| self.width() as f32);
        let height = bounds.height.unwrap_or_else(|| self.height() as f32);

        let x_offset = bounds.x - (bounds.anchor_x * width);
        let y_offset = bounds.y - (bounds.anchor_y * height);

        self.translate2(x_offset, y_offset);
    }

    pub fn translate2(&mut self, x_offset: f32, y_offset: f32) {
        self.translate3(f32::round(x_offset) as i32, f32::round(y_offset) as i32);
    }

    pub fn translate3(&mut self, x_offset: i32, y_offset: i32) {
        assert!(x_offset >= 0, "translate3: negative values not supported (unexpected negative)");
        assert!(y_offset >= 0, "translate3: negative values not supported (unexpected negative)");
        for line in self.0.iter_mut() {
            if !line.is_empty() {
                for _ in 0..x_offset {
                    line.insert(0, RenderCell::transparent());
                }
            }
        }
        for _ in 0..y_offset {
            self.0.insert(0, Vec::new());
        }
    }

    pub fn width(&self) -> usize {
        self.0.iter().map(|line| line.len()).max().unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.0.len()
    }

    pub fn collapse(layers: VRender<RenderLayer>) -> RenderLayer {
        let width = layers.iter().map(|layer| layer.width()).max().unwrap_or(0);
        let height = layers.iter().map(|layer| layer.height()).max().unwrap_or(0);

        let mut result = RenderLayer(vec![vec![RenderCell::transparent(); width]; height]);
        for layer in layers.into_iter() {
            for (y, layer_line) in layer.0.into_iter().enumerate() {
                let result_line = result.0.get_mut(y).unwrap();
                for (x, layer_cell) in layer_line.into_iter().enumerate() {
                    let result_cell = result_line.get_mut(x).unwrap();
                    if let RenderCellContent::TransparentChar = result_cell.content {
                        // Fall through
                        *result_cell = layer_cell;
                    } else if let RenderCellContent::TransparentCharWithZeroWidths { .. } = result_cell.content {
                        // Add prefix / suffix and fall through
                        // First we need to reassign result_cell so we can move prefix and suffix
                        let old_result_cell = mem::replace(result_cell, layer_cell);
                        let (prefix, suffix) = match old_result_cell.content {
                            RenderCellContent::TransparentCharWithZeroWidths { prefix, suffix } => (prefix, suffix),
                            _ => panic!("this can't happen")
                        };
                        result_cell.add_zw_prefix_suffix(prefix, suffix);
                    } else if !result_cell.bg.is_opaque() && !layer_cell.bg.is_transparent() {
                        // Add background color
                        result_cell.bg = PackedColor::stack(result_cell.bg, layer_cell.bg);
                    }
                }
            }
        }
        result
    }

    pub fn write(&self, output: &mut impl io::Write) -> io::Result<()> {
        IoWrite2FmtWrite::on(output, |output| {
            for (y, line) in self.0.iter().enumerate() {
                let mut prev_fg: PackedColor = PackedColor::transparent();
                let mut prev_bg: PackedColor = PackedColor::transparent();
                // Relative addressing leads to weird edge cases, especially with images or weird chars
                // This is set unless we're absolutely sure after writing, we're at the next (x, y) position
                // Like at the start and every line, we want to explicitly set the position because images do weird stuff with newlines
                let mut may_have_broken_position = true;
                let mut buffer = String::new();

                macro termctl($command:expr) {{
                    if !buffer.is_empty() {
                        output.write_str(buffer.as_str()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                        buffer.clear();
                    }
                    $command.write_ansi(output).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                }}

                for (x, cell) in line.iter().enumerate() {
                    // Fix position
                    if may_have_broken_position {
                        termctl!(cursor::MoveTo(x as u16, y as u16))?;
                        may_have_broken_position = false;
                    }

                    // Set foreground and background
                    if prev_fg != cell.fg && prev_bg != cell.bg {
                        termctl!(style::SetColors(style::Colors { foreground: Some(cell.fg.into()), background: Some(cell.bg.into()) }))?;
                    }
                    if prev_fg != cell.fg {
                        termctl!(style::SetForegroundColor(cell.fg.into()))?;
                    } else if prev_bg != cell.bg {
                        termctl!(style::SetBackgroundColor(cell.bg.into()))?;
                    }
                    prev_fg = cell.fg;
                    prev_bg = cell.bg;

                    match &cell.content {
                        RenderCellContent::TransparentChar => buffer.push(' '),
                        RenderCellContent::Char(char) => buffer.push(*char),
                        RenderCellContent::ManyChars(big_content) => {
                            buffer.push_str(big_content);
                            may_have_broken_position = true;
                        }
                        RenderCellContent::ZeroChars => {
                            may_have_broken_position = true;
                        }
                        RenderCellContent::TransparentCharWithZeroWidths { prefix, suffix } => {
                            buffer.push_str(prefix);
                            buffer.push(' ');
                            buffer.push_str(suffix);
                            may_have_broken_position = true;
                        }
                    }
                }

                // Reset colors (termctl will also print the buffer)
                termctl!(style::ResetColor)?;

                // Technically we don't have to write a newline because we move the position explicitly
                // HOWEVER some renderers (e.g. opening in a text editor) don't handle terminal escapes,
                // even in the case where we only expect to print to actual terminals.
                // Since it costs almost nothing to print newlines anyways, fixes confusing issues,
                // and makes debugging simple outputs easier, we do so.
                output.write_char('\n').map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            }

            // Just to make sure
            output.flush()?;

            Ok(())
        })
    }
}

impl Index<(usize, usize)> for RenderLayer {
    type Output = RenderCell;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.0[y][x]
    }
}

impl IndexMut<(usize, usize)> for RenderLayer {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        &mut self.0[y][x]
    }
}

impl From<Vec<Vec<RenderCell>>> for RenderLayer {
    fn from(lines: Vec<Vec<RenderCell>>) -> Self {
        Self::from_iter(lines.into_iter().map(|line| line.into_iter()))
    }
}

impl VRenderLayer for RenderLayer {
    fn clip(&mut self, clip_rect: &Rectangle) {
        for (y, line) in self.0.iter_mut().enumerate() {
            let y = y as f32;
            for (x, cell) in line.iter_mut().enumerate() {
                let x = x as f32;
                if x < clip_rect.left || x >= clip_rect.right || y < clip_rect.top || y >= clip_rect.bottom {
                    *cell = RenderCell::transparent();
                }
            }
        }
    }
}

impl Default for RenderLayer {
    fn default() -> Self {
        RenderLayer(Vec::new())
    }
}