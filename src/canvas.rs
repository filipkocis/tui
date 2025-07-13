use std::io::{self, Write, stdout};

use crossterm::{
    QueueableCommand, cursor,
    style::{self, Color},
};

use crate::{
    Code, Line, Padding, Size, Style, Viewport,
    text::{StyledUnit, Text},
};

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

    /// Get the width of the canvas (column width)
    pub fn width(&self) -> usize {
        self.buffer.iter().map(|l| l.width()).max().unwrap_or(0)
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

        for line in &text.visual {
            // let line = line.cutout(0, width);
            let line = Line {
                content: line.content.clone(),
            };
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
            if line.width() == 0 {
                continue;
            }

            line.set(0, StyledUnit::Code(Code::Background(color)));
            line.content
                .push(StyledUnit::Code(Code::Background(Color::Reset)));
        }
    }

    pub fn add_fg(&mut self, color: Option<Color>) {
        let color = match color {
            Some(color) => color,
            None => return,
        };

        for line in &mut self.buffer {
            if line.width() == 0 {
                continue;
            }

            line.set(0, StyledUnit::Code(Code::Foreground(color)));
            line.content
                .push(StyledUnit::Code(Code::Foreground(Color::Reset)));
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
                line.content.insert(0, StyledUnit::grapheme(" "));
            }

            for _ in 0..right {
                line.content.push(StyledUnit::grapheme(" "));
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
        let fg_reset_code = StyledUnit::Code(Code::Foreground(Color::Reset));

        let style_top = StyledUnit::grapheme("─");
        let style_bottom = StyledUnit::grapheme("─");
        let style_left = StyledUnit::grapheme("│");
        let style_right = StyledUnit::grapheme("│");

        let top_left = StyledUnit::grapheme("╭");
        let top_right = StyledUnit::grapheme("╮");
        let bottom_left = StyledUnit::grapheme("╰");
        let bottom_right = StyledUnit::grapheme("╯");

        let chars_width = if self.buffer.len() > 0 {
            self.buffer[0].width()
        } else {
            0
        };

        if has_top {
            let mut content = vec![style_top; chars_width];
            if let Some(color) = border_color {
                content.insert(0, StyledUnit::Code(Code::Foreground(color)));
                content.push(fg_reset_code.clone());
            }
            self.buffer.insert(0, Line { content });
        }

        if has_bottom {
            let mut content = vec![style_bottom; chars_width];
            if let Some(color) = border_color {
                content.insert(0, StyledUnit::Code(Code::Foreground(color)));
                content.push(fg_reset_code.clone());
            }
            self.buffer.push(Line { content });
        }

        let lines = self.buffer.len();
        if has_left {
            let first_char_index = border_color.is_some() as usize; // 0 has color code
            for (i, line) in self.buffer.iter_mut().enumerate() {
                if i == 0 && has_top {
                    line.content.insert(first_char_index, top_left.clone());
                } else if i == lines - 1 && has_bottom {
                    line.content.insert(first_char_index, bottom_left.clone());
                } else {
                    if let Some(color) = border_color {
                        line.content.insert(0, fg_reset_code.clone());
                        line.content.insert(0, style_left.clone());
                        line.content
                            .insert(0, StyledUnit::Code(Code::Foreground(color)));
                    } else {
                        line.content.insert(0, style_left.clone());
                    }
                }
            }
        }

        if has_right {
            let real_len_first = self.buffer.first().map(|l| l.content.len()).unwrap_or(0);
            let real_len_last = self.buffer.last().map(|l| l.content.len()).unwrap_or(0);

            let last_char_index_first = real_len_first - border_color.is_some() as usize; // real_len is reset code
            let last_char_index_last = real_len_last - border_color.is_some() as usize; // real_len is reset code

            for (i, line) in self.buffer.iter_mut().enumerate() {
                if i == 0 && has_top {
                    line.content
                        .insert(last_char_index_first, top_right.clone());
                } else if i == lines - 1 && has_bottom {
                    line.content
                        .insert(last_char_index_last, bottom_right.clone());
                } else {
                    if let Some(color) = border_color {
                        line.content.push(StyledUnit::Code(Code::Foreground(color)));
                        line.content.push(style_right.clone());
                        line.content.push(fg_reset_code.clone());
                    } else {
                        line.content.push(style_right.clone());
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
        is_first_extend: bool,
    ) {
        let child_width = child.width().min(style.size.width.computed_size() as usize);
        let max_height = style.size.height.computed_size() as usize;

        if style.size.width.computed_size() == 0 {
            return;
        }

        if style.flex_row {
            let gap_count = if include_gap { style.gap.0 as usize } else { 0 };
            let line_width = child_width + gap_count;

            // Setup the height of self so children can extend horizontally
            if is_first_extend {
                let blank_lines = (0..max_height).map(|_| Line::new(0));
                self.buffer.extend(blank_lines)
            }

            for i in 0..child.buffer.len() {
                let blank_line = Line::new(line_width);

                if i >= max_height {
                    break;
                }

                let line = &mut self.buffer[i];
                line.content.extend(blank_line.content);
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

            let x = self.position.0 as u16;
            stdout.queue(cursor::MoveTo(x, y as u16))?;

            for unit in &line.content {
                match unit {
                    StyledUnit::Grapheme(g) => {
                        stdout.queue(style::Print(&g.str))?;
                    }
                    StyledUnit::Code(code) => {
                        stdout.queue(style::Print(code))?;
                    }
                };
            }
        }

        stdout.queue(cursor::Show)?;
        stdout.flush()
    }

    /// Render self to `canvas` within the given `viewport`.
    pub fn render_to(&self, viewport: &Viewport, canvas: &mut Canvas) {
        for (i, line) in self.buffer.iter().enumerate() {
            let x = self.position.0;
            let y = self.position.1 + i as i16;

            if y >= 0 && y as u16 >= viewport.max.1 {
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

        debug_assert!(
            (y as usize) < self.buffer.len(),
            "Y position ({y}) is out of bounds ({}), you probably used the incorrect viewport, cannot paste on top of the canvas",
            self.buffer.len()
        );

        let start = (viewport.min.0 as i16 - x).max(0) as usize;
        let take = viewport.max.0.saturating_sub(viewport.min.0) as usize;
        let line = line.cutout(start, take);

        let column = (x as isize + start as isize).max(0) as usize;
        self.buffer[y as usize].paste_on_top(&line, column);
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

    fn line_ch(s: &str, n: usize) -> Line {
        Line {
            content: (0..n).map(|_| StyledUnit::grapheme(s)).collect(),
        }
    }

    fn line(v: StyledUnit, n: usize) -> Line {
        Line {
            content: (0..n).map(|_| v.clone()).collect(),
        }
    }

    #[test]
    fn paste_eq_len_no_col() {
        let vp = Viewport::new();
        let mut canvas = canvas(10, 10);
        let line = line_ch("a", 10);

        canvas.paste_on_top(&line, (0, 0), &vp);

        assert_eq!(line.content.len(), 10);
        assert_eq!(canvas.width(), 10);

        assert_eq!(canvas.buffer[0].content, line.content);
    }

    #[test]
    fn paste_eq_start_no_col() {
        let vp = Viewport::new();
        let mut canvas = canvas(10, 2);
        let line = line_ch("a", 5);

        canvas.paste_on_top(&line, (0, 0), &vp);

        assert_eq!(line.content.len(), 5);
        assert_eq!(canvas.width(), 10);

        assert_eq!(canvas.buffer[0].content[0..5], line.content);
        assert_eq!(canvas.buffer[0].content[5..], line_ch(" ", 5).content);
    }

    #[test]
    fn paste_middle() {
        let vp = Viewport::new();
        let mut canvas = canvas(5, 1);
        // canvas.buffer[0] = line_ch(['a', 'b', 'c', 'd', 'e']);

        let empty = StyledUnit::grapheme(" ");
        let letter = StyledUnit::grapheme("T");
        let mut line = line(letter.clone(), 1);

        let code = StyledUnit::Code(Code::Foreground(Color::Red));
        let ext = StyledUnit::Code(Code::Foreground(Color::Reset));
        line.set(0, code.clone());

        canvas.paste_on_top(&line, (2, 0), &vp);

        assert_eq!(line.content, [code.clone(), letter.clone()]);
        assert_eq!(
            canvas.buffer[0].content,
            [
                empty.clone(),
                empty.clone(),
                code.clone(),
                letter.clone(),
                ext.clone(),
                empty.clone(),
                empty.clone()
            ]
        );

        canvas.paste_on_top(&line, (2, 0), &vp);
        assert_eq!(line.content, [code.clone(), letter.clone()]);
        assert_ne!(
            canvas.buffer[0].content,
            [
                empty.clone(),
                empty.clone(),
                code.clone(),
                letter.clone(),
                ext.clone(),
                empty.clone(),
                empty.clone()
            ]
        );
        canvas.prune_redundant_codes();
        assert_eq!(
            canvas.buffer[0].content,
            [
                empty.clone(),
                empty.clone(),
                code.clone(),
                letter.clone(),
                ext.clone(),
                empty.clone(),
                empty.clone()
            ]
        );

        assert_eq!(line.content.len(), 2);
        assert_eq!(line.count(), 1);

        assert_eq!(canvas.width(), 5);
        assert_eq!(canvas.buffer[0].content.len(), 5 + 2);
        assert_eq!(canvas.buffer[0].count(), 5);
    }
}
