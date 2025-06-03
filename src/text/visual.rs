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
