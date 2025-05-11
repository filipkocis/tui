use std::io::{self, stdout, Write};

use crossterm::{
    cursor,
    style::{self, Color},
    QueueableCommand,
};

use crate::{Char, Code, Line, Size, Style, Viewport};

#[derive(Debug, Default)]
pub struct Canvas {
    /// Absolute position of the canvas
    pub position: (i16, i16),
    /// Content of the canvas
    pub buffer: Vec<Line>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        let mut buffer = Vec::with_capacity(height);

        for _ in 0..height {
            buffer.push(Line::new(width));
        }

        Self {
            position: (0, 0),
            buffer,
        }
    }

    /// True if absolute position `X, Y` is within the canvas
    pub fn hit_test(&self, x: i16, y: i16) -> bool {
        x > self.position.0
            && x < self.position.0 + self.width() as i16
            && y > self.position.1
            && y < self.position.1 + self.height() as i16
    }

    /// Get the width of the canvas
    pub fn width(&self) -> usize {
        self.buffer.iter().map(|line| line.len()).max().unwrap_or(0)
    }

    /// Get the height of the canvas
    pub fn height(&self) -> usize {
        self.buffer.len()
    }

    /// Normalize the canvas buffer size
    pub fn normalize(&mut self, style: &Style) {
        let mut max_len = self.width();
        let orig_height = self.height();
        let mut max_height = self.height();

        let width = style.size.width().computed_size() as usize;
        let height = style.size.height().computed_size() as usize;

        if style.grow {
            // Set to style size
            max_len = width;
            max_height = height;
        } else {
            // Take min size, bound by style size
            max_len = max_len.min(width);
            max_height = max_height.min(height);
        }

        if orig_height != max_height {
            self.buffer.resize_with(max_height, Default::default);
        }

        for line in &mut self.buffer {
            line.resize_to_fit(max_len);
        }
    }

    /// Add wrapped text
    pub fn add_text(&mut self, text: &str, size: Size) {
        let width = size.width().computed_size() as usize;
        let height = size.height().computed_size() as usize;

        if width == 0 || height == 0 || text.is_empty() {
            return;
        }

        let lines = text.split('\n');

        for line in lines {
            let part_count = line.len() / width + if line.len() % width > 0 { 1 } else { 0 };
            for i in 0..part_count {
                let start = i * width;
                let end = ((i + 1) * width).min(line.len());
                let part = &line[start..end];

                let chars = part.chars().map(Char::Char).collect();
                let line = Line { chars };
                if self.buffer.len() < height {
                    self.buffer.push(line);
                }
            }
        }
    }

    pub fn add_bg(&mut self, color: Option<Color>) {
        let color = match color {
            Some(color) => color,
            None => return,
        };

        for line in &mut self.buffer {
            line.set(0, Char::Code(Code::Background(color)));
            line.chars.push(Char::Code(Code::Background(Color::Reset)));
        }
    }

    pub fn add_fg(&mut self, color: Option<Color>) {
        let color = match color {
            Some(color) => color,
            None => return,
        };

        for line in &mut self.buffer {
            line.set(0, Char::Code(Code::Foreground(color)));
            line.chars.push(Char::Code(Code::Foreground(Color::Reset)));
        }
    }

    /// Add (top, bottom, left, right) padding
    pub fn add_padding(&mut self, padding: (u16, u16, u16, u16)) {
        let top = padding.0 as usize;
        let bottom = padding.1 as usize;
        let left = padding.2 as usize;
        let right = padding.3 as usize;

        let max_len = self.width();

        for _ in 0..top {
            self.buffer.insert(0, Line::new(max_len));
        }
        for _ in 0..bottom {
            self.buffer.push(Line::new(max_len));
        }

        for line in &mut self.buffer {
            for _ in 0..left {
                line.chars.insert(0, Char::Char(' '));
            }

            for _ in 0..right {
                line.chars.push(Char::Char(' '));
            }
        }
    }

    /// Add border
    pub fn add_border(&mut self, border: (bool, bool, bool, bool, Option<Color>)) {
        let has_top = border.0;
        let has_bottom = border.1;
        let has_left = border.2;
        let has_right = border.3;

        let border_color = border.4;

        let style_top = '─';
        let style_bottom = '─';
        let style_left = '│';
        let style_right = '│';

        let top_left = Char::Char('╭');
        let top_right = Char::Char('╮');
        let bottom_left = Char::Char('╰');
        let bottom_right = Char::Char('╯');

        let chars_len = if self.buffer.len() > 0 {
            self.buffer[0].len()
        } else {
            0
        };

        if has_top {
            let top_line = vec![style_top; chars_len];
            let chars = top_line.into_iter().map(Char::Char).collect();
            self.buffer.insert(0, Line { chars });
        }

        if has_bottom {
            let bottom_line = vec![style_bottom; chars_len];
            let chars = bottom_line.into_iter().map(Char::Char).collect();
            self.buffer.push(Line { chars });
        }

        let lines = self.buffer.len();
        if has_left {
            for (i, line) in self.buffer.iter_mut().enumerate() {
                if i == 0 && has_top {
                    line.chars.insert(0, Char::Char(style_left));
                } else if i == lines - 1 && has_bottom {
                    line.chars.insert(0, Char::Char(style_left));
                } else {
                    if let Some(color) = border_color {
                        line.chars
                            .insert(0, Char::Code(Code::Foreground(Color::Reset)));
                        line.chars.insert(0, Char::Char(style_left));
                        line.chars.insert(0, Char::Code(Code::Foreground(color)));
                    } else {
                        line.chars.insert(0, Char::Char(style_left));
                    }
                }
            }

            if has_top {
                self.buffer[0].set(0, top_left);
            }
            if has_bottom {
                self.buffer[lines - 1].set(0, bottom_left);
            }
        }

        if has_right {
            for line in &mut self.buffer {
                if let Some(color) = border_color {
                    line.chars.push(Char::Code(Code::Foreground(color)));
                    line.chars.push(Char::Char(style_right));
                    line.chars.push(Char::Code(Code::Foreground(Color::Reset)));
                } else {
                    line.chars.push(Char::Char(style_right));
                }
            }

            let chars_len = self.buffer[0].len() - 1;
            if has_top {
                let line = &mut self.buffer[0];
                line.set(chars_len, top_right);

                if let Some(color) = border_color {
                    line.set(0, Char::Code(Code::Foreground(color)));
                    line.chars.push(Char::Code(Code::Foreground(Color::Reset)));
                }
            }
            if has_bottom {
                let line = &mut self.buffer[lines - 1];
                line.set(chars_len, bottom_right);

                if let Some(color) = border_color {
                    line.set(0, Char::Code(Code::Foreground(color)));
                    line.chars.push(Char::Code(Code::Foreground(Color::Reset)));
                }
            }
        }
    }

    /// Extend the canvas with a blank copy of the child
    pub fn extend_child(
        &mut self,
        child: &Canvas,
        style: &Style,
        include_gap: bool,
        is_first_and_row: bool,
    ) {
        let child_width = child.width().min(style.size.width().computed_size() as usize);
        let max_height = style.size.height().computed_size() as usize;

        if style.size.width().computed_size() == 0 {
            return;
        }

        if style.flex_row {
            let gap_count = if include_gap { style.gap.0 as usize } else { 0 };
            let line_width = child_width + gap_count;

            for i in 0..child.buffer.len() {
                let blank_line = Line::new(line_width);

                if is_first_and_row {
                    if self.height() >= max_height {
                        break;
                    }

                    self.buffer.push(blank_line);
                } else {
                    if i >= max_height {
                        break;
                    }

                    let line = &mut self.buffer[i];
                    line.chars.extend(blank_line.chars);
                }
            }
        } else {
            let gap_count = if include_gap { style.gap.1 as usize } else { 0 };

            for _ in 0..child.buffer.len() + gap_count {
                if self.height() >= max_height {
                    break;
                }

                self.buffer.push(Line::new(child_width));
            }
        }
    }

    /// Render self to the screen.
    ///
    /// # Note
    /// It should be called on the root canvas only, children should use [`child.render_to(screen, root)`](Self::render_to)
    pub fn render(&self) -> io::Result<()> {
        let mut stdout = stdout();

        for (i, line) in self.buffer.iter().enumerate() {
            let y = self.position.1 + i as i16;
            if y < 0 {
                continue;
            }

            let print = style::Print(line);
            let goto = cursor::MoveTo(self.position.0 as u16, y as u16);
            stdout.queue(goto)?.queue(print)?;
        }

        stdout.flush()
    }

    /// Render self to `canvas` within the given `viewport`.
    pub fn render_to(&self, viewport: &Viewport, canvas: &mut Canvas) {
        for (i, line) in self.buffer.iter().enumerate() {
            let x = self.position.0;
            let y = self.position.1 + i as i16;

            if y as u16 >= viewport.max.1 {
                break; // Skip all lines, we are below the viewport
            }

            canvas.paste_on_top(&line, (x, y), viewport);
        }
    }

    /// Paste a line on top of the canvas at `position` within the `viewport`
    pub fn paste_on_top(&mut self, line: &Line, position: (i16, i16), viewport: &Viewport) {
        let x = position.0;
        let y = position.1;

        if y < viewport.min.1 as i16 || y as u16 >= viewport.max.1 {
            return; // Outside of the viewport
        }

        let start = (viewport.min.0 as i16 - x).max(0) as usize;
        let end = viewport.max.0.saturating_sub(viewport.min.0) as usize;
        let line = line.cutout(start, end);

        self.buffer[y as usize].paste_on_top(&line, x.max(0) as usize);
    }
}
