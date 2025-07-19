mod attrs;
mod buffer_line;
mod visual;

pub use attrs::*;
pub use buffer_line::*;
pub use visual::*;

use std::{ops::Range, path::Path};

use crate::{Code, code::CodeUnit};

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
    /// Text content, source of truth
    pub input: Vec<BufferLine>,
    pub visual: Vec<VisualLine>,
    pub styles: Vec<StyleSpan>,
    /// Text wrapping style
    pub wrap: TextWrap,
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
            visual: Vec::new(),
            styles: Vec::new(),
            wrap: TextWrap::default(),
            cursor: None,
        }
    }

    /// Wraps the visual text to the specified width. Must be called after `prepare_text`, calling
    /// this function multiple times will not have the desired effect.
    ///
    /// Returns the number of lines that were added due to wrapping.
    pub fn wrap_text(&mut self, width: u16) -> usize {
        let mut unwrapped_lines = Vec::new();
        let mut current_line = None;

        for line in self.visual.drain(..) {
            if line.offset == 0 {
                // If we have a line, push it
                if let Some(current) = current_line.take() {
                    unwrapped_lines.push(current);
                }

                // Start a new line
                current_line = Some(line);
                continue;
            }

            if let Some(current) = &mut current_line {
                // If we have a current line, append to it
                current.content.extend(line.content);
            } else {
                // If we don't have a current line, start a new one
                current_line = Some(line);
            }

            // TODO: implement code purging
            // line.purge();
        }

        // If we have a current line, push it
        if let Some(current) = current_line {
            unwrapped_lines.push(current);
        }

        let unwrapped_len = self.visual.len();

        self.visual = unwrapped_lines
            .into_iter()
            .flat_map(|line| line.into_wrapped(width))
            .collect();

        self.visual.len() - unwrapped_len
    }

    /// Prepares the text and apply styles
    /// Height will be clamped to the terminal size.
    pub fn prepare_text(&mut self, height: u16) {
        let skip = 0; // TODO: skip lines based on cursor position or other criteria
        let mut visual_lines = Vec::new();
        let terminal_height = crossterm::terminal::size().map_or(height, |(_, h)| h);
        let height = height.min(terminal_height);

        // Prepare styles
        self.prepare_styles();
        let mut styles = self.styles.iter().peekable();

        for (line_index, line) in self
            .input
            .iter()
            .enumerate()
            .skip(skip)
            .take(height as usize)
        {
            let grapheme_count = line.count();
            let mut visual_line = VisualLine::from_buffer_line(line, line_index);

            loop {
                // Skip styles that are no longer applicable
                if styles.peek().map_or(false, |s| s.line < line_index) {
                    styles.next();
                    continue;
                }

                // Break if the are no more styles for this line
                if styles.peek().map_or(false, |s| s.line > line_index) {
                    break;
                }

                let Some(style) = styles.next() else {
                    break;
                };

                // If the style starts after the line ends, skip it
                if style.character >= grapheme_count {
                    continue;
                }

                visual_line.add_style(style.code, style.character, style.length);
            }

            visual_lines.push(visual_line);
        }

        self.visual = visual_lines;
        self.sanitize();
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

        text.prepare_text(u16::MAX);
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

    /// Sanitizes the visual text
    pub fn sanitize(&mut self) {
        for line in &mut self.visual {
            for i in (0..line.content.len()).rev() {
                let StyledUnit::Grapheme(grapheme) = &line.content[i] else {
                    continue;
                };

                let Some(char) = grapheme.str.chars().next() else {
                    continue;
                };

                let (char_a, char_b) = match char {
                    '\u{0000}'..='\u{001F}' => ('^', (char as u8 + 0x40) as char),
                    '\u{007F}' => ('^', '?'),
                    _ => continue,
                };

                let grapheme_index = grapheme.grapheme_index;
                let grapheme_a = VisualGrapheme::new(char_a.to_string(), 1, grapheme_index);
                let grapheme_b = VisualGrapheme::new(char_b.to_string(), 1, grapheme_index);

                line.content[i] = StyledUnit::Grapheme(grapheme_b);
                line.content.insert(i, StyledUnit::Grapheme(grapheme_a));
            }
        }
    }

    /// Flattens styles into a contiguous non-overlapping array
    /// # Note
    /// Must be called after sorting styles
    fn flatten_styles(&mut self) {
        if self.styles.is_empty() {
            return;
        }

        /// Set a code range
        fn set(line: &mut Vec<CodeUnit>, code: Code, range: Range<usize>) {
            if line.len() < range.end {
                line.resize_with(range.end, || CodeUnit::default());
            }

            for i in range {
                match code {
                    Code::Attribute(attr) => line[i].apply_attr(attr),
                    Code::Background(bg) => line[i].set_bg(bg),
                    Code::Foreground(fg) => line[i].set_fg(fg),
                }
            }
        }

        /// Combine code units into style spans
        fn combine(line: &[CodeUnit], li: usize) -> Vec<StyleSpan> {
            let mut styles = vec![];

            // Active foreground color and start index
            let mut fg = None;
            let mut fg_i = 0;

            // Active background color and start index
            let mut bg = None;
            let mut bg_i = 0;

            // Currently active attributes and their start indices
            let mut attrs = Attrs::default().extract();
            let mut attrs_i = (0..attrs.len()).collect::<Vec<_>>();

            for i in 0..=line.len() {
                // Active codes at this index
                let unit = line.get(i);

                let unit_fg = unit.and_then(|u| u.fg());
                let unit_bg = unit.and_then(|u| u.bg());
                let unit_attrs = unit
                    .map(|u| u.attrs().extract())
                    .unwrap_or_else(|| Attrs::default().extract());

                if unit_fg != fg {
                    if let Some(fg) = fg {
                        styles.push(StyleSpan::new(Code::Foreground(fg), li, fg_i, i - fg_i));
                    }

                    fg = unit_fg;
                    fg_i = i;
                }

                if unit_bg != bg {
                    if let Some(bg) = bg {
                        styles.push(StyleSpan::new(Code::Background(bg), li, bg_i, i - bg_i));
                    }

                    bg = unit_bg;
                    bg_i = i;
                }

                for ai in 0..attrs.len() {
                    let attr = attrs[ai];
                    let unit_attr = unit_attrs[ai];
                    let attr_i = attrs_i[ai];

                    if unit_attr != attr {
                        if let Some(attr) = attr {
                            styles.push(StyleSpan::new(
                                Code::Attribute(attr),
                                li,
                                attr_i,
                                i - attr_i,
                            ));
                        }

                        attrs[ai] = unit_attr;
                        attrs_i[ai] = i;
                    }
                }
            }

            styles
        }

        let mut line = Vec::<CodeUnit>::new();
        let mut styles = vec![];
        let mut last_line = 0;
        for style in self.styles.drain(..) {
            if style.line < last_line {
                panic!("unsorted styles");
            }

            if style.line > last_line {
                styles.extend(combine(&line, last_line));
                last_line = style.line;
                line = Vec::new();
            }

            let range = style.character..style.end();
            set(&mut line, style.code, range);
        }

        if !line.is_empty() {
            styles.extend(combine(&line, last_line));
        }

        self.styles = styles;
    }

    /// Sort styles by index
    fn sort_styles(&mut self) {
        self.styles.sort_by(|a, b| {
            let line_cmp = a.line.cmp(&b.line);
            if line_cmp.is_eq() {
                a.character.cmp(&b.character)
            } else {
                line_cmp
            }
        });
    }

    /// Sort and flatten styles
    pub fn prepare_styles(&mut self) {
        self.sort_styles();
        self.flatten_styles();
    }

    /// Adds new styles to the existing one, re-prepares text
    pub fn add_styles(&mut self, style: Vec<StyleSpan>) {
        self.styles.extend(style);
        self.prepare_text(u16::MAX);
    }

    /// Returns visual text size `(width, height)` bound to terminal size.
    /// Height is the number of visual lines, width is the maximum column width of the lines.
    pub fn get_visual_size(&self) -> (u16, u16) {
        let (cols, rows) = crossterm::terminal::size().unwrap_or_default();

        let width = self.visual.iter().map(|l| l.width()).max().unwrap_or(0);
        let height = self.visual.len();

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
