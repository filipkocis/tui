use std::{collections::HashSet, fmt::Display};

use crossterm::style::{Attribute, Color};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    text::{StyledUnit, VisualGrapheme},
    Code,
};

#[derive(Debug, Default, Clone)]
pub struct Line {
    pub content: Vec<StyledUnit>,
}

impl Line {
    /// Returns a new line with `size` empty chars
    pub fn new(size: usize) -> Self {
        let content = vec![StyledUnit::grapheme(" "); size];
        Self { content }
    }

    /// Returns a line built from a string
    pub fn from_string(string: &str) -> Self {
        let content = string
            .graphemes(true)
            .map(|s| VisualGrapheme::new(s.to_string(), s.width(), None))
            .map(StyledUnit::Grapheme)
            .collect();
        Self { content }
    }

    /// Returns the `grapheme` count of the line
    pub fn count(&self) -> usize {
        self.content.iter().filter(|c| c.is_grapheme()).count()
    }

    /// Returns the `column` width of the line
    pub fn width(&self) -> usize {
        self.content.iter().map(|c| c.width()).count()
    }

    fn real_index(&self, index: usize) -> usize {
        let mut grapheme_count = 0;
        for (i, c) in self.content.iter().enumerate() {
            if c.is_grapheme() {
                if grapheme_count == index {
                    return i;
                }
                grapheme_count += 1;
            }
        }
        panic!("Index {index} {grapheme_count} out of bounds");
    }

    pub fn set(&mut self, index: usize, unit: StyledUnit) {
        let real_index = self.real_index(index);
        assert!(real_index < self.content.len());

        if unit.is_code() {
            self.content.insert(real_index, unit);
        } else {
            self.content[real_index] = unit;
        }
    }

    /// Returns all relevant codes active at `real_index`
    pub fn active_codes_at(&self, real_index: usize) -> Vec<Code> {
        let mut codes = HashSet::<Code>::new();
        let mut bg = None;
        let mut fg = None;

        for unit in &self.content[..real_index] {
            match unit {
                StyledUnit::Code(code) => match code {
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

    /// Returns a new line with the chars between `start` and `end` (exclusive).
    /// The new line will keep the correct codes at the start, all of them will be reset at the
    /// end.
    pub fn cutout(&self, start: usize, end: usize) -> Line {
        let line_len = self.count();
        let end = end.min(line_len);
        if start >= end || start as usize >= line_len {
            return Line::new(0);
        }

        let real_start = self.real_index(start);
        let real_end = self.real_index(end - 1);

        let active_codes = self
            .active_codes_at(real_start)
            .into_iter()
            .map(StyledUnit::Code);
        let reset_codes = self
            .reset_codes_for(real_end)
            .into_iter()
            .map(StyledUnit::Code);

        let mut line = Line::new(0);
        line.content.extend(active_codes);
        line.content
            .extend(self.content[real_start..=real_end].iter().cloned());
        line.content.extend(reset_codes);
        line
    }

    /// Paste another line on top of this one, starting at `start`.
    pub fn paste_on_top(&mut self, other: &Line, start: usize) {
        let other_len = other.count();
        if other_len == 0 {
            return;
        }

        let mut new_line = Line::new(0);

        if start != 0 {
            let real_start = self.real_index(start - 1);
            let reset_codes = self.reset_codes_for(real_start);
            new_line
                .content
                .extend(self.content[..=real_start].iter().cloned());
            new_line
                .content
                .extend(reset_codes.into_iter().map(StyledUnit::Code));
        }

        new_line.content.extend(other.content.iter().cloned());

        if start + other_len < self.count() {
            let real_end = self.real_index(start + other_len - 1);
            let active_codes = self.active_codes_at(real_end);
            new_line
                .content
                .extend(active_codes.into_iter().map(StyledUnit::Code));
            new_line
                .content
                .extend(self.content.iter().skip(real_end + 1).cloned());
        }

        self.content = new_line.content;
    }

    /// Prune redundant codes in the line, removing any codes with no effect, such as duplicates.
    pub fn prune_redundant_codes(&mut self) {
        // Active codes set previously in the line
        let mut set_attrs: Option<usize> = None;
        let mut set_fg = None;
        let mut set_bg = None;

        // Codes currently being processed before the next char
        let mut cur_attrs: Option<usize> = None;
        let mut cur_fg = None;
        let mut cur_bg = None;

        let mut content = Vec::new();

        for unit in self.content.drain(..) {
            match unit {
                // Set current code, but only if its not a first code == reset code
                StyledUnit::Code(code) => match code {
                    Code::Foreground(fg) => {
                        if set_fg.is_none() && fg == Color::Reset {
                            cur_fg = None;
                            continue;
                        }
                        cur_fg = Some(fg);
                    }
                    Code::Background(bg) => {
                        if set_bg.is_none() && bg == Color::Reset {
                            cur_bg = None;
                            continue;
                        }
                        cur_bg = Some(bg);
                    }
                    Code::Attribute(attr) => todo!("attr"),
                },
                // Consume current codes and apply them if they are different from set codes
                StyledUnit::Grapheme(grapheme) => {
                    if let Some(fg) = cur_fg.take() {
                        if set_fg != Some(fg) {
                            content.push(StyledUnit::Code(Code::Foreground(fg)));
                            set_fg = Some(fg);
                        }
                    }

                    if let Some(bg) = cur_bg.take() {
                        if set_bg != Some(bg) {
                            content.push(StyledUnit::Code(Code::Background(bg)));
                            set_bg = Some(bg);
                        }
                    }

                    if let Some(attrs) = cur_attrs.take() {
                        todo!("attrs")
                    }

                    content.push(StyledUnit::Grapheme(grapheme));
                }
            }
        }

        // Add final codes, only if they are reset codes and not first codes
        if content.len() != 0 {
            let reset = Some(Color::Reset);
            if reset == cur_fg.take() && set_fg != reset && set_fg.is_some() {
                content.push(StyledUnit::Code(Code::Foreground(Color::Reset)));
            }

            if reset == cur_bg.take() && set_bg != reset && set_bg.is_some() {
                content.push(StyledUnit::Code(Code::Background(Color::Reset)));
            }

            if let Some(attrs) = cur_attrs.take() {
                todo!("attrs")
            }
        }

        self.content = content;
        debug_assert_eq!(self.count() == 0, self.content.len() == 0);
    }

    /// Resize the line to fit exactly `width` in grapheme column width.
    pub fn resize_to_fit(&mut self, width: usize) {
        let mut diff = width as isize - self.count() as isize;

        if diff < 0 {
            // Pop `diff` graphemes
            while diff < 0 {
                match self.content.pop() {
                    Some(StyledUnit::Grapheme(_)) => diff += 1,
                    Some(StyledUnit::Code(_)) => continue,
                    None => break,
                }
            }

            // Remove trailing codes
            while self.content.last().map_or(false, |c| c.is_code()) {
                self.content.pop();
            }
        }

        // Add `diff` chars
        if diff > 0 {
            for _ in 0..diff {
                self.content.push(StyledUnit::grapheme(" "));
            }
        }
    }
}

impl Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for unit in &self.content {
            match unit {
                StyledUnit::Grapheme(g) => write!(f, "{}", g.str)?,
                StyledUnit::Code(code) => code.fmt(f)?,
            }
        }
        Ok(())
    }
}
