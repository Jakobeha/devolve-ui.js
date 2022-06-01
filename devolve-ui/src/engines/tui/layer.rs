// Thanks to http://xn--rpa.cc/irl/term.html for explaining obscure terminal escape codes and behaviors
use std::io;
use std::iter;
use std::fmt::Write;
use std::ops::{Index, IndexMut};
use crossterm::{Command, cursor};
use crossterm::style;
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
    ZeroChars
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
        assert!(char.width().is(1), "char width is not 1, use RenderCell::char() instead");
        RenderCell {
            content: RenderCellContent::Char(char),
            fg: fg,
            bg: bg
        }
    }

    pub fn char(char: char) -> impl Iterator<Item = Self> {
        assert!(char.width().unwrap_or(0) != 0, "char width is 0, handle explicitly");
        iter::once(Self::new(RenderCellContent::Char(char))).concat(
            (1..char.width()).map(|_i| Self::transparent())
        )
    }

    pub fn many_chars(chars: String) -> Self {
        Self::new(RenderCellContent::ManyChars(chars))
    }

    pub fn zero_width() -> Self {
        Self::new(RenderCellContent::ZeroChars)
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

    pub fn of(width: usize, height: usize, cell: RenderCell) -> Self {
        Self(vec![vec![cell; width]; height])
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
                    }
                }

                // Reset colors (termctl will also print the buffer)
                termctl!(style::ResetColor)?;
                // Instead of writing a newline, we move the position explicitly
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
        &self.0[x][y]
    }
}

impl IndexMut<(usize, usize)> for RenderLayer {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        &self.0[x][y]
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