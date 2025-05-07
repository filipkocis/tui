use std::{
    cell::RefCell,
    io::{stdout, Write},
    rc::Rc,
};

use crossterm::{
    cursor,
    style::{Color, SetBackgroundColor, SetForegroundColor},
    terminal,
};

use crate::offset::Offset;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub offset: Offset,
    pub size: (u16, u16),

    pub fg: Option<Color>,
    pub bg: Option<Color>,

    pub bold: bool,
    pub underline: bool,
    pub dim: bool,
    pub crossed: bool,

    pub padding: (u16, u16, u16, u16),
    pub border: (bool, bool, bool, bool, Option<Color>),

    pub flex_row: bool,
    pub grow: bool,
    pub gap: (u16, u16),
}

impl Style {
    pub fn apply(&self, other: &Style) -> Style {
        other.clone()
    }
}

#[derive(Debug, Default)]
pub struct Node {
    pub id: String,
    pub class: String,
    pub style: Style,
    pub content: String,
    pub parent: Option<Rc<RefCell<Node>>>,
    pub children: Vec<Rc<RefCell<Node>>>,
    pub focus: bool,

    canvas: Canvas,
}

#[derive(Debug, Clone)]
pub enum Char {
    Char(char),
    Code(String),
}

#[derive(Debug, Default)]
pub struct Line {
    pub chars: Vec<Char>,
}

impl Line {
    /// Returns the `char` length of the line
    pub fn len(&self) -> usize {
        self.chars
            .iter()
            .filter(|c| matches!(c, Char::Char(_)))
            .count()
    }

    fn real_index(&self, index: usize) -> usize {
        let mut real_index = 0;
        for (i, c) in self.chars.iter().enumerate() {
            if matches!(c, Char::Char(_)) {
                if real_index == index {
                    return i;
                }
                real_index += 1;
            }
        }
        panic!("Index {index} {real_index} out of bounds");
    }

    fn set(&mut self, index: usize, char: Char) {
        let real_index = self.real_index(index);
        assert!(real_index < self.chars.len());

        if matches!(char, Char::Code(_)) {
            self.chars.insert(real_index, char);
        } else {
            self.chars[real_index] = char;
        }
    }

    fn get(&self, index: usize) -> char {
        let real_index = self.real_index(index);
        if real_index < self.chars.len() {
            return match &self.chars[real_index] {
                Char::Char(c) => *c,
                _ => panic!("Not a char"),
            };
        }
        panic!("Index out of bounds");
    }

    /// Combines chars into a string, while skipping `start` Char::Chars and truncating to `end`
    fn combine(&self, mut start: u16, end: u16) -> String {
        let line_len = self.len();
        if start >= end || start as usize >= line_len {
            return String::new();
        }

        let mut result = String::new();
        let mut char_count = 0;
        let end = end as usize;
        let clear_colors = format!(
            "{}{}",
            SetBackgroundColor(Color::Reset),
            SetForegroundColor(Color::Reset)
        );

        for c in &self.chars {
            if matches!(c, Char::Char(_)) {
                char_count += 1;

                if char_count > end {
                    result.push_str(&clear_colors);
                    break;
                }

                if start > 0 {
                    start -= 1;
                    continue;
                }
            }

            match c {
                Char::Char(c) => result.push(*c),
                Char::Code(code) => result.push_str(code),
            };
        }

        result
    }

    /// Resize the line to fit exactly `len` in chars
    fn resize_to_fit(&mut self, len: usize) {
        let mut diff = len as isize - self.len() as isize;

        if diff < 0 {
            // Pop `diff` chars
            while diff < 0 {
                match self.chars.pop() {
                    Some(Char::Char(_)) => diff += 1,
                    Some(Char::Code(_)) => continue,
                    None => break,
                }
            }

            // Remove trailing codes
            while matches!(self.chars.last(), Some(&Char::Code(_))) {
                self.chars.pop();
            }
        }

        // Add `diff` chars
        if diff > 0 {
            for _ in 0..diff {
                self.chars.push(Char::Char(' '));
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Canvas {
    /// Absolute position of the canvas
    pub position: (i16, i16),
    /// Content of the canvas
    pub buffer: Vec<Line>,
}

impl Canvas {
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

        if style.grow {
            // Set to style size
            max_len = style.size.0 as usize;
            max_height = style.size.1 as usize;
        } else {
            // Take min size, bound by style size
            max_len = max_len.min(style.size.0 as usize);
            max_height = max_height.min(style.size.1 as usize);
        }

        if orig_height != max_height {
            self.buffer.resize_with(max_height, Default::default);
        }

        for line in &mut self.buffer {
            line.resize_to_fit(max_len);
        }
    }

    /// Add wrapped text
    pub fn add_text(&mut self, text: &str, size: (u16, u16)) {
        let width = size.0 as usize;
        let height = size.1 as usize;

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

        let set_bg_code = format!("{}", SetBackgroundColor(color));
        let clear_bg_code = format!("{}", SetBackgroundColor(Color::Reset));
        for line in &mut self.buffer {
            line.set(0, Char::Code(set_bg_code.clone()));
            line.chars.push(Char::Code(clear_bg_code.clone()));
        }
    }

    pub fn add_fg(&mut self, color: Option<Color>) {
        let color = match color {
            Some(color) => color,
            None => return,
        };

        let set_fg_code = format!("{}", SetForegroundColor(color));
        let clear_fg_code = format!("{}", SetForegroundColor(Color::Reset));
        for line in &mut self.buffer {
            line.set(0, Char::Code(set_fg_code.clone()));
            line.chars.push(Char::Code(clear_fg_code.clone()));
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
            self.buffer.insert(
                0,
                Line {
                    chars: vec![Char::Char(' '); max_len],
                },
            );
        }
        for _ in 0..bottom {
            self.buffer.push(Line {
                chars: vec![Char::Char(' '); max_len],
            });
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
        let set_fg_code = match border_color {
            Some(color) => format!("{}", SetForegroundColor(color)),
            None => String::new(),
        };
        let clear_fg_code = format!("{}", SetForegroundColor(Color::Reset));

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
                    line.chars.insert(0, Char::Code(clear_fg_code.clone()));
                    line.chars.insert(0, Char::Char(style_left));
                    line.chars.insert(0, Char::Code(set_fg_code.clone()));
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
                line.chars.push(Char::Code(set_fg_code.clone()));
                line.chars.push(Char::Char(style_right));
                line.chars.push(Char::Code(clear_fg_code.clone()));
            }

            let chars_len = self.buffer[0].len() - 1;
            if has_top {
                let line = &mut self.buffer[0];
                line.set(chars_len, top_right);

                if border_color.is_some() {
                    line.set(0, Char::Code(set_fg_code.clone()));
                    line.chars.push(Char::Code(clear_fg_code.clone()));
                }
            }
            if has_bottom {
                let line = &mut self.buffer[lines - 1];
                line.set(chars_len, bottom_right);

                if border_color.is_some() {
                    line.set(0, Char::Code(set_fg_code.clone()));
                    line.chars.push(Char::Code(clear_fg_code.clone()));
                }
            }
        }
    }

    /// Extend the canvas with a blank copy of the child
    pub fn extend_child(&mut self, child: &Canvas, style: &Style, include_gap: bool) {
        let max_height = style.size.1 as usize;

        if style.flex_row {
        } else {
            let child_width = child.width().min(style.size.0 as usize);
            let gap_count = if include_gap { style.gap.1 as usize } else { 0 };

            for _ in 0..child.buffer.len() + gap_count {
                if self.height() >= max_height {
                    break;
                }

                let blank_line = Line {
                    chars: vec![Char::Char(' '); child_width],
                };
                self.buffer.push(blank_line);
            }
        }
    }

    pub fn render(&self, viewport: &Viewport) {
        for (i, line) in self.buffer.iter().enumerate() {
            let y = self.position.1 + i as i16;
            if y < viewport.min.1 as i16 {
                continue; // Skip lines above the viewport
            }
            if y as u16 >= viewport.max.1 {
                break; // Skip lines below the viewport
            }

            let x = self.position.0;
            let start = (viewport.min.0 as i16 - x).max(0) as u16;
            let end = viewport.max.0 - viewport.min.0;

            let line = line.combine(start, end);
            let goto = cursor::MoveTo(x.max(0) as u16, y as u16);
            write!(stdout(), "{goto}{line}").unwrap();
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub min: (u16, u16),
    pub max: (u16, u16),
    pub screen: (u16, u16),
}

impl Viewport {
    pub fn new() -> Self {
        let (width, height) = terminal::size().unwrap();
        Self {
            min: (0, 0),
            max: (width, height),
            screen: (width, height),
        }
    }
}

impl Node {
    /// Max possible width of the node
    pub fn max_width(&self) -> u16 {
        self.style.size.0
            + self.style.padding.2
            + self.style.padding.3
            + self.style.border.2 as u16
            + self.style.border.3 as u16
    }

    /// Max possible height of the node
    pub fn max_height(&self) -> u16 {
        self.style.size.1
            + self.style.padding.0
            + self.style.padding.1
            + self.style.border.0 as u16
            + self.style.border.1 as u16
    }

    pub fn calculate_canvas(&mut self, parent_position: Offset) {
        let position = parent_position.add(self.style.offset);
        let content_position = position.add_tuple((
            self.style.padding.2 as i16 + self.style.border.2 as i16,
            self.style.padding.0 as i16 + self.style.border.0 as i16,
        ));

        let mut canvas = Canvas {
            position: position.tuple(),
            buffer: vec![],
        };

        let children_len = self.children.len();
        let mut extra_offset = (0, 0);
        for (i, child) in self.children.iter().enumerate() {
            let mut child = child.borrow_mut();
            child.calculate_canvas(content_position.add_tuple(extra_offset));

            if self.style.flex_row {
                extra_offset.0 += child.canvas.width() as i16 + self.style.gap.0 as i16;
            } else {
                extra_offset.1 += child.canvas.height() as i16 + self.style.gap.1 as i16;
            }

            canvas.extend_child(&child.canvas, &self.style, i < children_len - 1);
        }
        canvas.add_text(&self.content, self.style.size);
        canvas.normalize(&self.style);

        canvas.add_padding(self.style.padding);
        canvas.add_fg(self.style.fg);
        canvas.add_bg(self.style.bg);
        canvas.add_border(self.style.border);

        self.canvas = canvas;
    }

    pub fn render(&self, mut viewport: Viewport) {
        viewport.min = (
            self.canvas.position.0.max(0) as u16,
            self.canvas.position.1.max(0) as u16,
        );

        let max = (
            (self.canvas.position.0 + self.max_width() as i16).max(0) as u16,
            (self.canvas.position.1 + self.max_height() as i16).max(0) as u16,
        );

        let abs_max = if self.style.offset.is_absolute() {
            viewport.screen
        } else {
            viewport.max
        };

        viewport.max = (max.0.min(abs_max.0), max.1.min(abs_max.1));

        self.canvas.render(&viewport);

        if max.0 < viewport.screen.0 {
            viewport.max.0 -= self.style.padding.3 + self.style.border.3 as u16;
        }
        if max.1 < viewport.screen.1 {
            viewport.max.1 -= self.style.padding.1 + self.style.border.1 as u16;
        }

        for child in &self.children {
            let child = child.borrow();
            child.render(viewport);
        }

        stdout().flush().unwrap();
    }
}
