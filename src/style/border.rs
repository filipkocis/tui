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
    pub top: String,
    pub right: String,
    pub bottom: String,
    pub left: String,

    pub top_left: String,
    pub top_right: String,
    pub bottom_left: String,
    pub bottom_right: String,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self::sharp()
    }
}

impl BorderStyle {
    pub fn new<S: Into<String>>(
        top: S,
        right: S,
        bottom: S,
        left: S,
        top_left: S,
        top_right: S,
        bottom_left: S,
        bottom_right: S,
    ) -> Self {
        Self {
            top: top.into(),
            right: right.into(),
            bottom: bottom.into(),
            left: left.into(),
            top_left: top_left.into(),
            top_right: top_right.into(),
            bottom_left: bottom_left.into(),
            bottom_right: bottom_right.into(),
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
        Self::none()
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

    /// Sets the top side of the border
    #[inline]
    #[must_use]
    pub fn with_top(mut self, top: bool) -> Self {
        self.top = top;
        self
    }

    /// Sets the bottom side of the border
    #[inline]
    #[must_use]
    pub fn with_bottom(mut self, bottom: bool) -> Self {
        self.bottom = bottom;
        self
    }

    /// Sets the left side of the border
    #[inline]
    #[must_use]
    pub fn with_left(mut self, left: bool) -> Self {
        self.left = left;
        self
    }

    /// Sets the right side of the border
    #[inline]
    #[must_use]
    pub fn with_right(mut self, right: bool) -> Self {
        self.right = right;
        self
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
    #[inline]
    pub fn width(&self) -> u16 {
        self.left as u16 + self.right as u16
    }

    /// Returns the height of the border
    #[inline]
    pub fn height(&self) -> u16 {
        self.top as u16 + self.bottom as u16
    }

    /// Returns the size of the top border
    #[inline]
    pub fn top(&self) -> u16 {
        self.top as u16
    }

    /// Returns the size of the bottom border
    #[inline]
    pub fn bottom(&self) -> u16 {
        self.bottom as u16
    }

    /// Returns the size of the left border
    #[inline]
    pub fn left(&self) -> u16 {
        self.left as u16
    }

    /// Returns the size of the right border
    #[inline]
    pub fn right(&self) -> u16 {
        self.right as u16
    }
}
