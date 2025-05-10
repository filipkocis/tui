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

    /// Combines chars into a string, while skipping `start` Char::Chars and truncating to `end`
    pub fn combine(&self, mut start: u16, end: u16) -> String {
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
            if c.is_char() {
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
                Char::Code(code) => result.push_str(&code.to_string()),
            };
        }

        result
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
