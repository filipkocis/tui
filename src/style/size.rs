#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Defines a size value for a node.
pub enum SizeValue {
    /// Size determined by the contents of the node, the inner value is the computed size in cells
    Auto(u16),
    /// Size in cells
    Cells(u16),
    /// Percentage of the parent node, or the viewport. The second value is the computed size in
    /// cells
    Percent(u16, u16),
}

impl Default for SizeValue {
    fn default() -> Self {
        SizeValue::Auto(0)
    }
}

impl SizeValue {
    /// Parses a string into a [`SizeValue`]
    /// Valid values are `auto, 50%, 42`
    pub fn parse(value: &str) -> Option<Self> {
        if value == "auto" {
            Some(Self::auto())
        } else if value.ends_with('%') {
            let percent = &value[..value.len() - 1];
            percent.parse::<u16>().ok().map(|v| Self::percent(v))
        } else if value.matches(char::is_numeric).count() == value.len() {
            value.parse::<u16>().ok().map(|v| Self::Cells(v))
        } else {
            None
        }
    }

    #[inline]
    pub fn auto() -> Self {
        SizeValue::Auto(0)
    }

    #[inline]
    pub fn percent(value: u16) -> Self {
        SizeValue::Percent(value, 0)
    }

    /// Gets the computed size in cells.
    #[inline]
    pub fn computed_size(self) -> u16 {
        match self {
            Self::Auto(v) => v,
            Self::Cells(v) => v,
            Self::Percent(_, v) => v,
        }
    }

    /// Sets the computed size in cells for the current size value.
    #[inline]
    pub fn set_computed_size(self, value: u16) -> Self {
        match self {
            Self::Auto(_) => Self::Auto(value),
            Self::Cells(_) => Self::Cells(value),
            Self::Percent(p, _) => Self::Percent(p, value),
        }
    }

    #[inline]
    pub fn is_auto(self) -> bool {
        matches!(self, SizeValue::Auto(_))
    }

    #[inline]
    pub fn is_cells(self) -> bool {
        matches!(self, SizeValue::Cells(_))
    }

    #[inline]
    pub fn is_percent(self) -> bool {
        matches!(self, SizeValue::Percent(..))
    }
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
/// Represents the size of a node's style, defined by its width and height.
pub struct Size(pub SizeValue, pub SizeValue);

impl Size {
    /// Parses strings into a [`Size`]
    pub fn parse(width: &str, height: &str) -> Option<Self> {
        let width = SizeValue::parse(width)?;
        let height = SizeValue::parse(height)?;
        Self::new(width, height).into()
    }

    #[inline]
    pub fn new(width: SizeValue, height: SizeValue) -> Self {
        Size(width, height)
    }

    #[inline]
    pub fn new_from(width: impl Into<SizeValue>, height: impl Into<SizeValue>) -> Self {
        Size(width.into(), height.into())
    }

    #[inline]
    pub fn from_cells(width: u16, height: u16) -> Self {
        Size(SizeValue::Cells(width), SizeValue::Cells(height))
    }

    #[inline]
    pub fn from_percent(width: u16, height: u16) -> Self {
        Size(SizeValue::Percent(width, 0), SizeValue::Percent(height, 0))
    }

    #[inline]
    pub fn width(&self) -> SizeValue {
        self.0
    }

    #[inline]
    pub fn height(&self) -> SizeValue {
        self.1
    }
}
