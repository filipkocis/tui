use std::sync::OnceLock;

use crossterm::style::Color;

static DEFAULT_BORDER: OnceLock<BorderStyle> = OnceLock::new();

/// Returns the default border style.
pub fn get_default_border() -> &'static BorderStyle {
    DEFAULT_BORDER.get_or_init(|| BorderStyle::default())
}

// Call this once before any use to override the default border style.
pub fn set_default_border(border: BorderStyle) -> Result<(), BorderStyle> {
    DEFAULT_BORDER.set(border)
}

/// Border style for nodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderStyle {
    pub top: char,
    pub right: char,
    pub bottom: char,
    pub left: char,

    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self::sharp()
    }
}

impl BorderStyle {
    pub fn new(
        top: char,
        right: char,
        bottom: char,
        left: char,
        top_left: char,
        top_right: char,
        bottom_left: char,
        bottom_right: char,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    /// Border with sharp corners
    pub fn sharp() -> Self {
        Self::new('─', '│', '─', '│', '┌', '┐', '└', '┘')
    }

    /// Border with rounded corners
    pub fn rounded() -> Self {
        Self::new('─', '│', '─', '│', '╭', '╮', '╰', '╯')
    }

    /// Border with no corners or edges
    pub fn empty() -> Self {
        Self::new(' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ')
    }
}

/// Defines whether a border side is enabled or not
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Border {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
    pub color: Option<Color>,
    pub style: &'static BorderStyle,
}

impl Default for Border {
    fn default() -> Self {
        Self::from_sides(false, false, false, false)
    }
}

impl Border {
    /// Creates a new border with specified enabled sides, color, and style
    #[inline]
    pub fn new(
        top: bool,
        right: bool,
        bottom: bool,
        left: bool,
        color: Option<Color>,
        style: &'static BorderStyle,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
            color,
            style,
        }
    }

    /// New border with specified sides enabled
    #[inline]
    pub fn from_sides(top: bool, right: bool, bottom: bool, left: bool) -> Self {
        Self::new(top, right, bottom, left, None, get_default_border())
    }

    /// All sides enabled
    #[inline]
    pub fn all() -> Self {
        Self::from_sides(true, true, true, true)
    }

    /// All sides disabled
    #[inline]
    pub fn none() -> Self {
        Self::from_sides(false, false, false, false)
    }

    /// Only horizontal sides enabled
    #[inline]
    pub fn horizontal() -> Self {
        Self::from_sides(true, false, true, false)
    }

    /// Only vertical sides enabled
    #[inline]
    pub fn vertical() -> Self {
        Self::from_sides(false, true, false, true)
    }

    /// Sets the color of the border
    #[inline]
    #[must_use]
    pub fn with_color(mut self, color: Option<Color>) -> Self {
        self.color = color;
        self
    }

    /// Sets the style of the border
    #[inline]
    #[must_use]
    pub fn with_style(mut self, style: &'static BorderStyle) -> Self {
        self.style = style;
        self
    }

    /// Returns the width of the border
    pub fn width(&self) -> u16 {
        self.left as u16 + self.right as u16
    }

    /// Returns the height of the border
    pub fn height(&self) -> u16 {
        self.top as u16 + self.bottom as u16
    }
}
