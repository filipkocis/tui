use unicode_segmentation::{GraphemeIndices, Graphemes, UnicodeSegmentation};
use unicode_width::UnicodeWidthStr;

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
