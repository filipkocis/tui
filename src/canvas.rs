use std::io::{self, stdout, Write};

use crossterm::{
    cursor,
    style::{self, Color},
    QueueableCommand,
};

use crate::{text::Text, Char, Code, Line, Padding, Size, Style, Viewport};

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
        x >= self.position.0
            && x < self.position.0 + self.width() as i16
            && y >= self.position.1
            && y < self.position.1 + self.height() as i16
    }

    /// Get the width of the canvas
    pub fn width(&self) -> usize {
        self.buffer.iter().map(|line| line.len()).max().unwrap_or(0)
    }

    /// Get the height of the canvas
    #[inline]
    pub fn height(&self) -> usize {
        self.buffer.len()
    }

    /// Normalize the canvas buffer size
    pub fn normalize(&mut self, style: &Style) {
        let canvas_height = self.height();

        let width = style.size.width.computed_size() as usize;
        let height = style.size.height.computed_size() as usize;

        if canvas_height != height {
            self.buffer.resize_with(height, Default::default);
        }

        for line in &mut self.buffer {
            line.resize_to_fit(width);
        }
    }

    /// Add wrapped text
    pub fn add_text(&mut self, text: &Text, size: Size) {
        let width = size.width.computed_size() as usize;
        let height = size.height.computed_size() as usize;

        if width == 0 || height == 0 {
            return;
        }

        // TODO: Have finalized be just for the viewport
        for line in &text.finalized {
            // let line = line.cutout(0, width);
            let line = line.clone();
            // if self.buffer.len() < height {
            self.buffer.push(line);
            // }
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
    pub fn add_padding(&mut self, padding: Padding) {
        let top = padding.top as usize;
        let bottom = padding.bottom as usize;
        let left = padding.left as usize;
        let right = padding.right as usize;

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
        let fg_reset_code = Char::Code(Code::Foreground(Color::Reset));

        let style_top = Char::Char('─');
        let style_bottom = Char::Char('─');
        let style_left = Char::Char('│');
        let style_right = Char::Char('│');

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
            let mut chars = vec![style_top; chars_len];
            if let Some(color) = border_color {
                chars.insert(0, Char::Code(Code::Foreground(color)));
                chars.push(fg_reset_code);
            }
            self.buffer.insert(0, Line { chars });
        }

        if has_bottom {
            let mut chars = vec![style_bottom; chars_len];
            if let Some(color) = border_color {
                chars.insert(0, Char::Code(Code::Foreground(color)));
                chars.push(fg_reset_code);
            }
            self.buffer.push(Line { chars });
        }

        let lines = self.buffer.len();
        if has_left {
            let first_char_index = border_color.is_some() as usize; // 0 has color code
            for (i, line) in self.buffer.iter_mut().enumerate() {
                if i == 0 && has_top {
                    line.chars.insert(first_char_index, top_left);
                } else if i == lines - 1 && has_bottom {
                    line.chars.insert(first_char_index, bottom_left);
                } else {
                    if let Some(color) = border_color {
                        line.chars.insert(0, fg_reset_code);
                        line.chars.insert(0, style_left);
                        line.chars.insert(0, Char::Code(Code::Foreground(color)));
                    } else {
                        line.chars.insert(0, style_left);
                    }
                }
            }
        }

        if has_right {
            let real_len = self.buffer.get(0).map(|l| l.chars.len()).unwrap_or(0);
            let last_char_index = real_len - border_color.is_some() as usize; // real_len is reset code
            for (i, line) in self.buffer.iter_mut().enumerate() {
                if i == 0 && has_top {
                    line.chars.insert(last_char_index, top_right);
                } else if i == lines - 1 && has_bottom {
                    line.chars.insert(last_char_index, bottom_right);
                } else {
                    if let Some(color) = border_color {
                        line.chars.push(Char::Code(Code::Foreground(color)));
                        line.chars.push(style_right);
                        line.chars.push(fg_reset_code);
                    } else {
                        line.chars.push(style_right);
                    }
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
        let child_width = child.width().min(style.size.width.computed_size() as usize);
        let max_height = style.size.height.computed_size() as usize;

        if style.size.width.computed_size() == 0 {
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
        stdout.queue(cursor::Hide)?;

        for (i, line) in self.buffer.iter().enumerate() {
            let y = self.position.1 + i as i16;
            if y < 0 {
                continue;
            }

            let print = style::Print(line);
            let goto = cursor::MoveTo(self.position.0 as u16, y as u16);
            stdout.queue(goto)?.queue(print)?;
        }

        stdout.queue(cursor::Show)?;
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

    /// Prune redundant codes from the canvas, removing any codes that are not needed and have no
    /// effect, like duplicates.
    pub fn prune_redundant_codes(&mut self) {
        for line in &mut self.buffer {
            line.prune_redundant_codes();
        }
    }
}

#[cfg(test)]
mod canvas {
    use super::*;

    fn canvas(w: usize, h: usize) -> Canvas {
        Canvas::new(w, h)
    }

    fn line_ch<const N: usize>(v: [char; N]) -> Line {
        Line {
            chars: v.map(Char::Char).to_vec(),
        }
    }

    fn line<const N: usize>(v: [Char; N]) -> Line {
        Line { chars: v.to_vec() }
    }

    #[test]
    fn paste_eq_len_no_col() {
        let vp = Viewport::new();
        let mut canvas = canvas(10, 10);
        let line = line([Char::Char('a'); 10]);

        canvas.paste_on_top(&line, (0, 0), &vp);

        assert_eq!(line.chars.len(), 10);
        assert_eq!(canvas.width(), 10);

        assert_eq!(canvas.buffer[0].chars, line.chars);
    }

    #[test]
    fn paste_eq_start_no_col() {
        let vp = Viewport::new();
        let mut canvas = canvas(10, 2);
        let line = line([Char::Char('a'); 5]);

        canvas.paste_on_top(&line, (0, 0), &vp);

        assert_eq!(line.chars.len(), 5);
        assert_eq!(canvas.width(), 10);

        assert_eq!(canvas.buffer[0].chars[0..5], line.chars);
        assert_eq!(canvas.buffer[0].chars[5..], [Char::Char(' '); 5]);
    }

    #[test]
    fn paste_middle() {
        let vp = Viewport::new();
        let mut canvas = canvas(5, 1);
        // canvas.buffer[0] = line_ch(['a', 'b', 'c', 'd', 'e']);

        let empty = Char::Char(' ');
        let letter = Char::Char('T');
        let mut line = line([letter]);

        let code = Char::Code(Code::Foreground(Color::Red));
        let ext = Char::Code(Code::Foreground(Color::Reset));
        line.set(0, code);

        canvas.paste_on_top(&line, (2, 0), &vp);

        assert_eq!(line.chars, [code, letter]);
        assert_eq!(
            canvas.buffer[0].chars,
            [empty, empty, code, letter, ext, empty, empty]
        );

        canvas.paste_on_top(&line, (2, 0), &vp);
        assert_eq!(line.chars, [code, letter]);
        assert_ne!(
            canvas.buffer[0].chars,
            [empty, empty, code, letter, ext, empty, empty]
        );
        canvas.prune_redundant_codes();
        assert_eq!(
            canvas.buffer[0].chars,
            [empty, empty, code, letter, ext, empty, empty]
        );

        assert_eq!(line.chars.len(), 2);
        assert_eq!(line.len(), 1);

        assert_eq!(canvas.width(), 5);
        assert_eq!(canvas.buffer[0].chars.len(), 5 + 2);
        assert_eq!(canvas.buffer[0].len(), 5);
    }
}
