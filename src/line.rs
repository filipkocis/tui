use std::{collections::HashSet, fmt::Display};

use crossterm::style::{Attribute, Color};

use crate::{Char, Code};

#[derive(Debug, Default)]
pub struct Line {
    pub chars: Vec<Char>,
}

impl Line {
    pub fn new(size: usize) -> Self {
        let chars = vec![Char::Char(' '); size];
        Self { chars }
    }

    /// Returns the `char` length of the line
    pub fn len(&self) -> usize {
        self.chars.iter().filter(|c| c.is_char()).count()
    }

    fn real_index(&self, index: usize) -> usize {
        let mut char_count = 0;
        for (i, c) in self.chars.iter().enumerate() {
            if c.is_char() {
                if char_count == index {
                    return i;
                }
                char_count += 1;
            }
        }
        panic!("Index {index} {char_count} out of bounds");
    }

    pub fn set(&mut self, index: usize, char: Char) {
        let real_index = self.real_index(index);
        assert!(real_index < self.chars.len());

        if char.is_code() {
            self.chars.insert(real_index, char);
        } else {
            self.chars[real_index] = char;
        }
    }

    /// Returns all relevant codes active at `real_index`
    pub fn active_codes_at(&self, real_index: usize) -> Vec<Code> {
        let mut codes = HashSet::<Code>::new();
        let mut bg = None;
        let mut fg = None;

        for c in &self.chars[..real_index] {
            match c {
                Char::Code(code) => match code {
                    Code::Attribute(attr) => {
                        if *attr == Attribute::Reset {
                            codes.retain(|c| !c.is_attribute())
                        } else {
                            codes.insert(code.clone());
                        }
                    }
                    Code::Background(color) => {
                        if *color == Color::Reset {
                            bg = None;
                        } else {
                            bg = Some(*color);
                        }
                    }
                    Code::Foreground(color) => {
                        if *color == Color::Reset {
                            fg = None;
                        } else {
                            fg = Some(*color);
                        }
                    }
                },
                _ => {}
            }
        }

        bg.map(|color| codes.insert(Code::Background(color)));
        fg.map(|color| codes.insert(Code::Foreground(color)));
        codes.into_iter().collect()
    }

    /// Returns all relevant reset codes to end the style at `real_index`
    pub fn reset_codes_for(&self, real_index: usize) -> Vec<Code> {
        let active_codes = self.active_codes_at(real_index);
        let mut reset_codes = Vec::new();
        let mut has_attr = false;

        for code in active_codes {
            if code.is_reset() {
                continue;
            }

            if code.is_attribute() {
                has_attr = true;
            } else {
                reset_codes.push(code.into_reset());
            }
        }

        if has_attr {
            reset_codes.push(Code::Attribute(Attribute::Reset));
        }

        reset_codes
    }

    /// Resize the line to fit exactly `len` in chars
    pub fn resize_to_fit(&mut self, len: usize) {
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
            while self.chars.last().map_or(false, |c| c.is_code()) {
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

impl Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in &self.chars {
            match c {
                Char::Char(c) => write!(f, "{}", c)?,
                Char::Code(code) => code.fmt(f)?,
            }
        }
        Ok(())
    }
}
