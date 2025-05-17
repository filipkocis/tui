use std::path::Path;

use crate::{Char, Code, Line};

#[derive(Debug)]
/// Defines the input type for a text, stores the loaded content split into lines
pub enum TextInput {
    /// Plain text input
    Plain(Vec<String>),
    /// Text input from a file
    File(String, Vec<String>),
}

impl TextInput {
    pub fn get_lines(&self) -> &[String] {
        match self {
            TextInput::Plain(lines) => lines,
            TextInput::File(_, lines) => lines,
        }
    }
}

impl Default for TextInput {
    fn default() -> Self {
        TextInput::Plain(Vec::new())
    }
}

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

#[derive(Debug, Default)]
/// Text object with styles and wrapping
pub struct Text {
    /// The original text content, source of truth
    pub input: TextInput,
    /// Color and attribute commands
    pub style: Vec<CodeSpan>,
    /// Text with applied styles and sanitization
    pub processed: Vec<Line>,
    /// Visible and wrapped text
    pub finalized: Vec<Line>,

    /// Size of the orignal text (width, height)
    pub original_size: (usize, usize),
    /// Size of the processed text
    pub processed_size: (usize, usize),

    /// Text wrapping style
    pub wrap: TextWrap,
}

impl Text {
    fn new_from(input: TextInput, size: (usize, usize)) -> Self {
        Self {
            input,
            style: Vec::new(),
            processed: Vec::new(),
            finalized: Vec::new(),
            original_size: size,
            processed_size: (0, 0),
            wrap: TextWrap::default(),
        }
    }

    /// Creates a text object from a string
    pub fn plain(input: &str) -> Self {
        let lines = input.lines();
        let lines = lines.map(|line| line.to_string()).collect::<Vec<_>>();
        let size_total = (
            lines.iter().map(|l| l.chars().count()).max().unwrap_or(0),
            lines.len(),
        );

        let mut text = Self::new_from(TextInput::Plain(lines), size_total);

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

        let lines = std::fs::read_to_string(&absolute_path)
            .map_err(|_| Error::new(ErrorKind::Other, "Failed to read file"))?
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();

        let size_total = (
            lines.iter().map(|l| l.chars().count()).max().unwrap_or(0),
            lines.len(),
        );

        let mut text = Self::new_from(TextInput::File(absolute_path, lines), size_total);

        text.process_text();
        Ok(text)
    }

    /// Process the text and apply styles
    pub fn process_text(&mut self) {
        let lines = self.input.get_lines();
        let mut processed = Vec::new();

        // let mut codes = self.style.iter();
        // let mut current_code = codes.next();
        let mut current_code_index = 0;
        let mut current_index = 0;

        for line in lines {
            let mut large_code_span_index = None;
            let mut processed_line = Line::from_string(line);
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

                if parts == 1 {
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
