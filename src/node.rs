use std::{
    cell::RefCell,
    io::{stdout, Write},
    rc::Rc,
};

use crossterm::style::{self, Color, SetBackgroundColor, SetForegroundColor};

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub offset: (u16, u16),
    pub size: (u16, u16),

    pub fg: Option<style::Color>,
    pub bg: Option<style::Color>,

    pub bold: bool,
    pub underline: bool,
    pub dim: bool,
    pub crossed: bool,

    pub padding: (u16, u16, u16, u16),
    pub border: (bool, bool, bool, bool, Option<Color>),

    pub flex_row: bool,
    pub grow: bool,
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
}

#[derive(Debug, Clone)]
pub enum Char {
    Char(char),
    Code(String),
}

#[derive(Default)]
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

    fn combine(&self) -> String {
        self.chars
            .iter()
            .map(|c| match c {
                Char::Char(c) => c.to_string(),
                Char::Code(code) => code.clone(),
            })
            .collect()
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

pub struct Canvas {
    pub position: (u16, u16),
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
    pub fn extend_child(&mut self, child: &Canvas, style: &Style) {
        let max_height = style.size.1 as usize;

        if style.flex_row {
        } else {
            let child_width = child.width().min(style.size.0 as usize);

            for _ in 0..child.buffer.len() {
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

    pub fn render(&self) {
        for (i, line) in self.buffer.iter().enumerate() {
            let line = line.combine();
            let goto = crossterm::cursor::MoveTo(self.position.0, self.position.1 + i as u16);
            write!(stdout(), "{goto}{line}").unwrap();
        }
    }
}

impl Node {
    pub fn width(&self) -> usize {
        self.style.size.0 as usize
            + self.style.padding.0 as usize
            + self.style.padding.2 as usize
            + self.style.border.0 as usize
            + self.style.border.1 as usize
    }

    pub fn height(&self) -> usize {
        self.style.size.1 as usize
            + self.style.padding.1 as usize
            + self.style.padding.3 as usize
            + self.style.border.2 as usize
            + self.style.border.3 as usize
    }

    pub fn calculate_canvas(&self) -> Canvas {
        let mut canvas = Canvas {
            position: self.style.offset,
            buffer: vec![],
        };

        for child in &self.children {
            let child = child.borrow();
            let child_canvas = child.calculate_canvas();
            canvas.extend_child(child_canvas, &self.style);
        }
        canvas.add_text(&self.content, self.style.size);
        canvas.normalize(&self.style);

        canvas.add_padding(self.style.padding);
        canvas.add_fg(self.style.fg);
        canvas.add_bg(self.style.bg);
        canvas.add_border(self.style.border);

        canvas
    }

    pub fn render(&self) {
        let canvas = self.calculate_canvas();
        canvas.render();

        for child in &self.children {}

        stdout().flush().unwrap();
    }
}
