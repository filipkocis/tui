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

#[derive(Debug, Clone)]
pub struct VisualGrapheme {
    /// The grapheme as a string, can be a normal string but needs to have a valid width
    str: String,
    /// The visual width of the grapheme in columns
    width: usize,
    /// This grapheme's grapheme index in the original line. Used in conjunction with
    /// `line_index` from [`VisualLine`].
    /// It will be `None` if the grapheme is not a part of the original line.
    grapheme_index: Option<usize>,
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
