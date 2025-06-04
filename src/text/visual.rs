use unicode_width::UnicodeWidthStr;

use crate::{Char, Code, Line};

use super::BufferLine;

#[derive(Debug, Clone)]
pub struct StyleSpan {
    /// The style code to apply
    pub code: Code,
    /// The line index in the text
    pub line: usize,
    /// The grapheme index in the line
    pub character: usize,
    /// The length of the style span in graphemes
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VisualGrapheme {
    /// The grapheme as a string, can be a normal string but needs to have a valid width
    pub str: String,
    /// The visual width of the grapheme in columns
    pub width: usize,
    /// This grapheme's grapheme index in the original line. Used in conjunction with
    /// `line_index` from [`VisualLine`].
    /// It will be `None` if the grapheme is not a part of the original line.
    pub grapheme_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StyledUnit {
    Grapheme(VisualGrapheme),
    Code(Code),
}

#[derive(Debug, Clone)]
pub struct VisualLine {
    pub content: Vec<StyledUnit>,
    /// The original line index in the text
    pub line_index: usize,
    /// Grapheme offset in the original line
    pub offset: usize,
}

impl StyleSpan {
    /// Creates a new style span
    pub fn new(code: Code, line: usize, character: usize, length: usize) -> Self {
        Self {
            code,
            line,
            character,
            length,
        }
    }
}

impl VisualGrapheme {
    #[inline]
    pub fn new(str: String, width: usize, index: Option<usize>) -> Self {
        Self {
            str,
            width,
            grapheme_index: index,
        }
    }
}

impl StyledUnit {
    /// Returns the width of the styled unit
    pub fn width(&self) -> usize {
        match self {
            StyledUnit::Grapheme(g) => g.width,
            StyledUnit::Code(_) => 0, // Codes have no visual width
        }
    }

    /// Returns if the styled unit is a `Grapheme` variant
    #[inline]
    pub fn is_grapheme(&self) -> bool {
        matches!(self, StyledUnit::Grapheme(_))
    }

    /// Returns if the styled unit is a `Code` variant
    #[inline]
    pub fn is_code(&self) -> bool {
        matches!(self, StyledUnit::Code(_))
    }

    /// Returns a grapheme created from a string, string must be a valid grapheme
    #[inline]
    pub fn grapheme(str: &str) -> Self {
        StyledUnit::Grapheme(VisualGrapheme::new(str.to_string(), str.width(), None))
    }
}

impl VisualLine {
    pub fn from_buffer_line(line: &BufferLine, line_index: usize) -> Self {
        let mut content = Vec::new();

        for (grapheme_index, (byte_index, byte_length, width)) in
            line.grapheme_data().into_iter().enumerate()
        {
            let grapheme = line.content()[*byte_index..*byte_index + *byte_length].to_string();
            let visual_grapheme = VisualGrapheme::new(grapheme, *width, Some(grapheme_index));

            content.push(StyledUnit::Grapheme(visual_grapheme))
        }

        Self {
            content,
            line_index,
            offset: 0,
        }
    }

    /// Returns the column width of the line
    pub fn width(&self) -> usize {
        self.content.iter().map(|unit| unit.width()).sum()
    }

    /// Returns a `vec` of visual lines, which are wrapped parts of this line.
    pub fn into_wrapped(self, max_width: u16) -> Vec<Self> {
        let mut lines = Vec::new();
        let line_index = self.line_index;
        let mut offset = self.offset;

        let width = self.width();
        let parts = (width as f32 / max_width.max(1) as f32).ceil() as usize;
        if parts <= 1 {
            // If the line fits in the max width, return it as is
            return vec![self];
        }

        if max_width == 0 {
            // If the max width is 0, return empty lines
            for _ in 0..parts {
                lines.push(Self {
                    content: Vec::new(),
                    line_index,
                    offset,
                });

                offset += 1; // Increment offset for each new line
            }
            return lines;
        }

        let mut content = self.content.into_iter().peekable();

        for _ in 0..parts {
            let mut line_content = Vec::new();
            let mut line_width = 0;

            while let Some(unit) = content.peek() {
                let unit_width = unit.width();
                if line_width + unit_width > max_width as usize {
                    // If adding this unit exceeds the max width, break
                    break;
                }

                // Add the unit to the line content
                let unit = content.next().expect("Peeked unit should be present");
                if unit.is_grapheme() {
                    offset += 1;
                }

                line_content.push(unit);
                line_width += unit_width;
            }

            // Create a new visual line from the collected content
            lines.push(Self {
                content: line_content,
                line_index,
                offset,
            });
        }

        lines
    }

    /// Returns the last grapheme index in the line, if any
    fn last_grapheme_index(&self) -> Option<usize> {
        self.content.iter().rev().find_map(|unit| {
            if let StyledUnit::Grapheme(g) = unit {
                g.grapheme_index
            } else {
                None
            }
        })
    }

    /// Adds a style to this line in the grapheme index range (inclusive, exlusive)
    pub fn add_style(&mut self, code: Code, character: usize, length: usize) {
        // If the style has no length, skip it
        if length == 0 {
            return;
        }

        let Some(grapheme_count) = self.last_grapheme_index() else {
            // If there are no graphemes, skip the style
            return;
        };

        // If the character is out of bounds, skip it
        if character >= grapheme_count {
            return;
        }

        let character_end = if character + length >= grapheme_count {
            grapheme_count
        } else {
            character + length
        };

        self.add_code(code, character);
        self.add_code(code.into_reset(), character_end);
    }

    /// Adds a code at the specified grapheme index
    pub fn add_code(&mut self, code: Code, grapheme_index: usize) {
        // Insert the code at the grapheme index
        let position = self.get_position(grapheme_index);
        self.content.insert(position, StyledUnit::Code(code));
    }

    /// Returns the index `self.content[index]` of the nearest grapheme matching `grapheme_index`
    pub fn get_position(&self, grapheme_index: usize) -> usize {
        let mut last_index = 0;
        for (i, unit) in self.content.iter().enumerate() {
            if let StyledUnit::Grapheme(g) = unit {
                if let Some(index) = g.grapheme_index {
                    if index == grapheme_index {
                        return i;
                    }
                    last_index = i;
                }
            }
        }

        // If the grapheme index is not found, return the last index
        last_index
    }
}
