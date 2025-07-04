use std::{collections::HashSet, fmt::Display};

use crossterm::style::{Attribute, Color};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{
    Code,
    text::{StyledUnit, VisualGrapheme},
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
        self.content.iter().map(|c| c.width()).sum()
    }

    /// Returns the byte index of the grapheme at `column`.
    /// The second value is the `column start` of the grapheme.
    /// Third value is the grapheme's `width`.
    ///
    /// Panics if the column is out of bounds.
    fn column_to_index(&self, column: usize) -> (usize, usize, usize) {
        let mut columns = 0;
        for (i, c) in self.content.iter().enumerate() {
            if c.is_grapheme() {
                let width = c.width();
                if columns + width > column {
                    return (i, columns, width);
                }
                columns += width;
            }
        }
        panic!("Column {column} out of bounds, max is {columns}");
    }

    /// Set a grapheme at `column` to `unit`.
    /// If unit is a code, it will be inserted at the grapheme's index, otherwise it will replace
    /// the grapheme.
    pub fn set(&mut self, column: usize, unit: StyledUnit) {
        let (real_index, ..) = self.column_to_index(column);
        debug_assert!(real_index < self.content.len());

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

    /// Returns a new line with the graphemes between `(column..column + length)` column range, if
    /// a column is not a grapheme boundary, it will be filled with spaces.
    /// The new line will keep the correct codes at the start, all of them will be reset at the
    /// end.
    pub fn cutout(&self, column: usize, length: usize) -> Line {
        let line_width = self.width();
        let length = length.min(line_width.saturating_sub(column));

        if length == 0 || column >= line_width {
            return Line::new(0);
        }

        let (start_index, start_column, start_width) = self.column_to_index(column);
        let (end_index, end_column, end_width) = self.column_to_index(column + length - 1);
        let start_gap = start_width - (column - start_column);
        let end_gap = (column + length) - end_column;

        let active_codes = self
            .active_codes_at(start_index)
            .into_iter()
            .map(StyledUnit::Code);
        let reset_codes = self
            .reset_codes_for(end_index)
            .into_iter()
            .map(StyledUnit::Code);

        let mut line = Line::new(0);

        // Start line with active codes
        line.content.extend(active_codes);

        // Fill with spaces, if start is inside a grapheme
        if start_gap < start_width {
            debug_assert!(start_gap > 0, "start_gap should always be greater than 0");
            line.content
                .extend((0..start_gap).map(|_| StyledUnit::grapheme(" ")));
        }

        let content_range = {
            let start_index = if start_width > 1 && start_gap < start_width {
                start_index + 1
            } else {
                start_index
            };

            if end_width > 1 && end_gap < end_width {
                &self.content[start_index..end_index]
            } else {
                &self.content[start_index..=end_index]
            }
        };

        // Extend with the grapheme range
        line.content.extend(content_range.iter().cloned());

        // Fill with spaces, if end is inside a grapheme
        if end_gap > 0 && end_gap < end_width {
            line.content
                .extend((0..end_gap).map(|_| StyledUnit::grapheme(" ")));
        }

        // End line with reset codes
        line.content.extend(reset_codes);
        line
    }

    /// Paste another line on top of this one, starting at `column`.
    pub fn paste_on_top(&mut self, other: &Line, column: usize) {
        let other_width = other.width();
        if other_width == 0 {
            return;
        }

        debug_assert!(
            column <= self.width(),
            "Column {column} out of bounds, width is {}, other width is {}. Cannot paste on top of line after it's end.",
            self.width(),
            other.width()
        );

        let mut new_line = Line::new(0);

        // Add original line content up to `column`
        if column != 0 {
            let (start_index, start_column, start_width) = self.column_to_index(column - 1);
            let start_gap = column - start_column;

            let start_slice = if start_width > 1 && start_gap < start_width {
                &self.content[..start_index]
            } else {
                &self.content[..=start_index]
            };

            // Add content
            new_line.content.extend(start_slice.iter().cloned());

            // Fill with spaces if inside a grapheme
            if start_gap > 0 && start_gap < start_width {
                new_line
                    .content
                    .extend((0..start_gap).map(|_| StyledUnit::grapheme(" ")));
            }

            // End with reset codes
            let reset_codes = self.reset_codes_for(start_index);
            new_line
                .content
                .extend(reset_codes.into_iter().map(StyledUnit::Code));
        }

        // Add the other line content
        new_line.content.extend(other.content.iter().cloned());

        // Add original line content after `column + other_width`
        if column + other_width < self.width() {
            // let real_end = self.real_index(start + other_len - 1);
            let (end_index, end_column, end_width) = self.column_to_index(column + other_width - 1);

            // Add active codes for the end
            let active_codes = self.active_codes_at(end_index);
            new_line
                .content
                .extend(active_codes.into_iter().map(StyledUnit::Code));

            // Fill with spaces if inside a grapheme
            let end_gap = end_width - (column + other_width - end_column);
            if end_gap < end_width && end_gap > 0 {
                new_line
                    .content
                    .extend((0..end_gap).map(|_| StyledUnit::grapheme(" ")));
            }

            // Add content after the end
            new_line
                .content
                .extend(self.content.iter().skip(end_index + 1).cloned());
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
        debug_assert_eq!(
            self.count() == 0,
            self.content.len() == 0,
            "Line content {} should be empty if and only if grapheme count {} is 0",
            self.content.len(),
            self.count()
        );
    }

    /// Resize the line to fit exactly `width` in grapheme column width.
    pub fn resize_to_fit(&mut self, width: usize) {
        let mut diff = width as isize - self.width() as isize;

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

#[cfg(test)]
mod paste_tests {
    use super::*;

    fn paste(line: &str, other: &str, column: usize, expected: &str) {
        let mut line = Line::from_string(line);
        let original_line = line.clone();
        let other = Line::from_string(other);
        line.paste_on_top(&other, column);
        assert_eq!(
            line.to_string(),
            expected,
            "\nPasting '{}' on top of '{}' at column {} should result in '{}'",
            other.to_string(),
            original_line.to_string(),
            column,
            expected
        );
    }

    #[test]
    fn normal_start() {
        let line = "1234";
        let other = "hello";

        paste(line, other, 0, "hello");
        paste(line, other, 1, "1hello");
        paste(line, other, 2, "12hello");
    }

    #[test]
    fn width_start() {
        let line = "❤️❤️";
        let other = "hello";

        paste(line, other, 0, "hello");
        paste(line, other, 1, " hello");
        paste(line, other, 2, "❤️hello");
        paste(line, other, 3, "❤️ hello");
        paste(line, other, 4, "❤️❤️hello");
    }

    #[test]
    fn normal_full() {
        let line = "123456";
        let other = "hello";

        paste(line, other, 0, "hello6");
        paste(line, other, 1, "1hello");
        paste(line, other, 2, "12hello");
    }

    #[test]
    fn width_full() {
        let line = "❤️❤️ ❤️❤️";
        let other = "hello";

        paste(line, other, 0, "hello❤️❤️");
        paste(line, other, 1, " hello ❤️");
        paste(line, other, 2, "❤️hello❤️");
        paste(line, other, 3, "❤️ hello ");
        paste(line, other, 4, "❤️❤️hello");
    }

    #[test]
    fn width_end() {
        let line = "12❤️";
        let other = "hi";

        paste("❤️", "h", 0, "h ");
        paste(line, other, 0, "hi❤️");
        paste(line, other, 1, "1hi ");
        paste(line, other, 2, "12hi");
        paste(line, other, 3, "12 hi");
        paste(line, other, 4, "12❤️hi");
    }
}

#[cfg(test)]
mod cutout_tests {
    use super::*;

    fn cutout(line: &str, column: usize, length: usize, expected: &str) {
        let line = Line::from_string(line);
        let cutout = line.cutout(column, length);
        assert_eq!(
            cutout.to_string(),
            expected,
            "\nCutting out '{}' at column {} with length {} should result in '{}'",
            line.to_string(),
            column,
            length,
            expected
        );
    }

    #[test]
    fn normal() {
        let line = "1234567890";
        cutout(line, 0, 0, "");
        cutout(line, 0, 5, "12345");
        cutout(line, 2, 3, "345");
        cutout(line, 5, 4, "6789");
        cutout(line, 8, 10, "90");
        cutout(line, 10, 10, "");
        cutout(line, 20, 10, "");
    }

    #[test]
    fn width() {
        let line = "❤️❤️❤️❤️";
        cutout(line, 0, 0, "");
        cutout(line, 0, 1, " ");
        cutout(line, 0, 2, "❤️");
        cutout(line, 0, 3, "❤️ ");
        cutout(line, 1, 4, " ❤️ ");
        cutout(line, 1, 5, " ❤️❤️");
        cutout(line, 1, 6, " ❤️❤️ ");
        cutout(line, 2, 4, "❤️❤️");
        cutout(line, 2, 5, "❤️❤️ ");
        cutout(line, 7, 4, " ");
        cutout(line, 10, 50, "");
    }
}
