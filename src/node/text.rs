use std::path::Path;

use unicode_segmentation::{GraphemeIndices, Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

use crate::{Char, Code, Line};

#[derive(Debug)]
/// Defines a style for a text span from start (inclusive) to end (exclusive)
pub struct CodeSpan {
    pub code: Code,
    pub start: usize,
    pub end: usize,
}

impl CodeSpan {
    /// Creates a new text code
    pub fn new(code: Code, start: usize, end: usize) -> Self {
        Self { code, start, end }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
/// Text wrapping options
pub enum TextWrap {
    /// No wrapping
    None,
    /// Break at whitespace
    Word,
    /// Break at character
    #[default]
    All,
}

#[derive(Debug, Clone, Default)]
/// A line in the text buffer
pub struct BufferLine {
    /// The content of the line
    content: String,
    /// Data about graphemes in the line (byte_index, byte_length, width)
    grapheme_data: Vec<(usize, usize, usize)>,
}

impl BufferLine {
    /// Creates a new buffer line from string
    pub fn new(content: String) -> Self {
        let mut buffer_line = Self {
            content,
            grapheme_data: Vec::new(),
        };
        buffer_line.recalculate_grapheme_data();
        buffer_line
    }

    /// Recalculates `grapheme_data` for the line
    fn recalculate_grapheme_data(&mut self) {
        self.grapheme_data = self
            .content
            .grapheme_indices(true)
            .map(|(i, d)| (i, d.len(), d.width()))
            .collect()
    }

    /// Returns graphemes via `unicode-segmentation` crate
    #[inline]
    pub fn graphemes(&self) -> Graphemes {
        self.content.graphemes(true)
    }

    /// Returns grapheme indices via `unicode_segmentation` crate
    #[inline]
    pub fn grapheme_indices(&self) -> GraphemeIndices {
        self.content.grapheme_indices(true)
    }

    /// Reference to the line content
    #[inline]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Consumes self and returns the line content
    #[inline]
    pub fn into_content(self) -> String {
        self.content
    }

    /// Returns the grapheme data for the line, each tuple contains:
    /// - `byte_index` - the byte index of the grapheme in the line
    /// - `byte_length` - the byte length of the grapheme
    /// - `width` - the visual/column width of the grapheme
    #[inline]
    pub fn grapheme_data(&self) -> &[(usize, usize, usize)] {
        &self.grapheme_data
    }

    /// Returns the number of graphemes in the line
    /// **Not** the same as the `colume width`
    #[inline]
    pub fn count(&self) -> usize {
        self.grapheme_data.len()
    }

    /// Returns the column width of the line, calculated from grapheme data
    /// **Not** the same as the `grapheme count`
    pub fn width(&self) -> usize {
        self.grapheme_data.iter().map(|(.., w)| w).sum()
    }

    /// Returns byte index of the grapheme at the given index, None if the index is out of bounds
    pub fn grapheme_to_byte_index(&self, grapheme_index: usize) -> Option<usize> {
        self.grapheme_data
            .get(grapheme_index)
            .map(|(byte_index, ..)| *byte_index)
    }

    /// Returns grapheme index for the byte index, None if the byte index is out of bounds or not a
    /// grapheme start
    pub fn byte_to_grapheme_index(&self, byte_index: usize) -> Option<usize> {
        self.grapheme_data
            .iter()
            .position(|(index, ..)| *index == byte_index)
    }

    /// Replaces a **byte** range in the line with a new string
    pub fn replace_range(&mut self, range: impl std::ops::RangeBounds<usize>, replace_with: &str) {
        self.content.replace_range(range, replace_with);
        self.recalculate_grapheme_data();
    }

    /// Removes a slice from this line at the specified **byte** range, and returns it.
    pub fn remove_range(&mut self, range: impl std::ops::RangeBounds<usize>) -> String {
        let removed_content = self.content.drain(range).collect();
        self.recalculate_grapheme_data();
        removed_content
    }

    /// Removes a grapheme at the specified **grapheme** index, and returns it.
    pub fn remove(&mut self, grapheme_index: usize) -> String {
        let grapheme_range = self
            .grapheme_data
            .get(grapheme_index)
            .map(|(start, length, _)| *start..*start + *length)
            .expect("Grapheme index out of bounds, cannot remove grapheme");

        let removed_content = self.content.drain(grapheme_range).collect();
        self.recalculate_grapheme_data();
        removed_content
    }

    /// Inserts a string at the specified byte index
    pub fn insert_str(&mut self, index: usize, string: &str) {
        self.content.insert_str(index, string);
        self.recalculate_grapheme_data();
    }

    /// Inserts a character at the specified byte index
    pub fn insert(&mut self, index: usize, char: char) {
        self.content.insert(index, char);
        self.recalculate_grapheme_data();
    }

    /// Pushes a string to the end of the line
    pub fn push_str(&mut self, string: &str) {
        self.content.push_str(string);
        self.recalculate_grapheme_data();
    }

    /// Pushes a character to the end of the line
    pub fn push(&mut self, char: char) {
        self.content.push(char);
        self.recalculate_grapheme_data();
    }

    /// Returns a slice of the line content based on the specified grapheme range (end exclusive).
    pub fn slice(&self, start: usize, end: usize) -> Option<&str> {
        let start = self.grapheme_to_byte_index(start)?;
        let end = if end >= self.count() {
            self.content.len()
        } else {
            self.grapheme_to_byte_index(end)?
        };

        self.content.get(start..end)
    }
}

#[derive(Debug, Default)]
/// Text object with styles and wrapping
pub struct Text {
    /// Text content, source of truth
    pub input: Vec<BufferLine>,
    /// Color and attribute commands
    pub style: Vec<CodeSpan>,
    /// Text wrapping style
    pub wrap: TextWrap,

    /// Text with applied styles and sanitization
    pub processed: Vec<Line>,
    /// Visible and wrapped text
    pub finalized: Vec<Line>,

    // /// Size of the orignal text (width, height)
    // pub original_size: (usize, usize),
    /// Size of the processed text
    pub processed_size: (usize, usize),

    /// Cursor position in the text, used for rendering
    pub cursor: Option<(u16, u16)>,
}

impl Text {
    fn new_from(input: Vec<BufferLine>) -> Self {
        // let original_size = (
        //     input.iter().map(|l| l.chars().count()).max().unwrap_or(0),
        //     input.len(),
        // );

        Self {
            input,
            style: Vec::new(),
            processed: Vec::new(),
            finalized: Vec::new(),
            // original_size,
            processed_size: (0, 0),
            wrap: TextWrap::default(),
            cursor: None,
        }
    }

    /// Creates a text object from a string
    pub fn plain(input: &str) -> Self {
        let mut lines = input
            .lines()
            .map(|line| BufferLine::new(line.to_string()))
            .collect::<Vec<_>>();

        if input.ends_with('\n') || input.is_empty() {
            lines.push(BufferLine::default());
        }

        let mut text = Self::new_from(lines);

        text.process_text();
        text
    }

    /// Creates a text object from file at `path`
    pub fn file(path: &str) -> std::io::Result<Self> {
        use std::io::{Error, ErrorKind};

        let path = Path::new(path).canonicalize()?;
        let absolute_path = path
            .into_os_string()
            .into_string()
            .map_err(|_| Error::new(ErrorKind::Other, "Invalid UTF-8 in path"))?;

        let content = std::fs::read_to_string(&absolute_path)
            .map_err(|_| Error::new(ErrorKind::Other, "Failed to read file"))?;

        Ok(Self::plain(&content))
    }

    /// Process the text and apply styles
    pub fn process_text(&mut self) {
        let mut processed = Vec::new();

        // let mut codes = self.style.iter();
        // let mut current_code = codes.next();
        let mut current_code_index = 0;
        let mut current_index = 0;

        for line in &self.input {
            let mut large_code_span_index = None;
            let mut processed_line = Line::from_string(line.content());
            let line_len = processed_line.len();
            let line_end_index = current_index + line_len;

            while let Some(code) = self.style.get(current_code_index) {
                debug_assert!(
                    code.end >= code.start,
                    "Code end must be greater than or equal to start"
                );

                if code.end < current_index {
                    // Skip codes that are already processe
                    current_code_index += 1;
                    continue;
                }

                if code.start >= line_end_index {
                    // Break if code isn't active yet
                    break;
                } else {
                    // Activate the code, it's either active for this line, or since previous lines
                    let index = code.start.saturating_sub(current_index);
                    processed_line.set(index, Char::Code(code.code));
                }

                if code.end <= line_end_index {
                    // Deactivate the code, it has ended on this line
                    let reset_code = Char::Code(code.code.into_reset());
                    if code.end == line_end_index {
                        processed_line.chars.push(reset_code);
                    } else {
                        processed_line.set(code.end - current_index, reset_code);
                    }
                } else {
                    // The code is active for more lines, so we reset this line's end
                    processed_line
                        .chars
                        .push(Char::Code(code.code.into_reset()));
                    // Store the first idnex of a code that spans multiple lines
                    if large_code_span_index.is_none() {
                        large_code_span_index = Some(current_code_index);
                    }
                }

                // Move to the next code
                current_code_index += 1;
            }

            if let Some(index) = large_code_span_index {
                // Move code index back to the last code index which spans multiple lines
                current_code_index = index;
            }

            processed.push(processed_line);
            current_index += line_len + 1; // +1 for the newline character
        }

        self.processed = processed;
        self.sanitize();

        self.processed_size = (
            self.processed.iter().map(|l| l.len()).max().unwrap_or(0),
            self.processed.len(),
        );
    }

    /// Finalize the text, apply wrapping and other operations
    pub fn finalize_text(&mut self, max_width: u16) {
        if self.wrap == TextWrap::None || self.get_processed_size().0 <= max_width {
            // No need for wrapping
            self.finalized = self.processed.clone();
            return;
        }

        if self.wrap == TextWrap::All {
            let mut finalized = Vec::new();

            for line in &self.processed {
                let line_len = line.len();
                let parts = (line_len as f32 / max_width.max(1) as f32).ceil() as usize;

                if parts <= 1 {
                    finalized.push(line.clone());
                    continue;
                }

                if max_width == 0 {
                    for _ in 0..parts {
                        finalized.push(Line::new(0));
                    }
                    continue;
                }

                let mut start = 0;
                let mut end = max_width as usize;
                for _ in 0..parts {
                    if end > line_len {
                        end = line_len;
                    }

                    let cutout = line.cutout(start, end);
                    finalized.push(cutout);

                    start = end;
                    end += max_width as usize;

                    if start >= line_len {
                        break;
                    }
                }
            }

            self.finalized = finalized;
        }

        if self.wrap == TextWrap::Word {
            // TODO: word wrap
            unimplemented!("word wrap");

            // let mut finalized = Vec::new();
            //
            // for line in &self.processed {
            //     let mut start = 0;
            //     let mut end = max_width as usize;
            // }
            //
            // self.finalized = finalized
        }
    }

    /// Sanitizes the processed text
    pub fn sanitize(&mut self) {
        for line in &mut self.processed {
            for i in (0..line.chars.len()).rev() {
                let Char::Char(char) = line.chars[i] else {
                    continue;
                };

                let (char_a, char_b) = match char {
                    '\u{0000}'..='\u{001F}' => ('^', (char as u8 + 0x40) as char),
                    '\u{007F}' => ('^', '?'),
                    _ => continue,
                };

                line.chars[i] = Char::Char(char_b);
                line.chars.insert(i, Char::Char(char_a));
            }
        }
    }

    /// Sort styles by index
    pub fn sort_style(&mut self) {
        self.style.sort_by(|a, b| {
            if a.start == b.start {
                a.end.cmp(&b.end)
            } else {
                a.start.cmp(&b.start)
            }
        });
    }

    /// Adds new style to the existing one, re-processes text
    pub fn add_style(&mut self, style: Vec<CodeSpan>) {
        self.style.extend(style);
        self.sort_style();
        self.process_text();
    }

    /// Sets new style, re-processes text
    pub fn set_style(&mut self, style: Vec<CodeSpan>) {
        // TODO: flatten, prune and sort the styles
        self.style = style;
        self.sort_style();
        self.process_text();
    }

    /// Get the current style
    pub fn style(&self) -> &[CodeSpan] {
        &self.style
    }

    /// Returns processed text size `(width, height)` bound to terminal size.
    pub fn get_processed_size(&self) -> (u16, u16) {
        let (width, height) = self.processed_size;
        let (cols, rows) = crossterm::terminal::size().unwrap_or_default();

        let width = width.min(cols as usize) as u16;
        let height = height.min(rows as usize) as u16;

        (width, height)
    }
}

impl Into<Text> for &str {
    fn into(self) -> Text {
        Text::plain(self)
    }
}

impl Into<Text> for String {
    fn into(self) -> Text {
        Text::plain(&self)
    }
}
