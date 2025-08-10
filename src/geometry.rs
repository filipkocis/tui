//! Geometry-related types and traits.

/// Rectangular area defined by its minimum and maximum x and y coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub min_x: u16,
    pub max_x: u16,
    pub min_y: u16,
    pub max_y: u16,
}

/// Partial rectangular area where each coordinate can be `None`, indicating that the edge is
/// not defiend or is open-ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PartialRect {
    pub min_x: Option<u16>,
    pub max_x: Option<u16>,
    pub min_y: Option<u16>,
    pub max_y: Option<u16>,
}

impl Rect {
    /// Creates a new rectangle with the given coordinates.
    #[inline]
    pub fn new(min_x: u16, max_x: u16, min_y: u16, max_y: u16) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    /// Checks if a point is inside the rectangle.
    #[inline]
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.min_x && x < self.max_x && y >= self.min_y && y < self.max_y
    }
}

impl PartialRect {
    /// Creates a new partial rectangle with the given coordinates.
    #[inline]
    pub fn new(
        min_x: Option<u16>,
        max_x: Option<u16>,
        min_y: Option<u16>,
        max_y: Option<u16>,
    ) -> Self {
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    /// Creates a new partial rectangle from a full rectangle.
    #[inline]
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            min_x: Some(rect.min_x),
            max_x: Some(rect.max_x),
            min_y: Some(rect.min_y),
            max_y: Some(rect.max_y),
        }
    }

    /// Creates a new partial rectangle with only the **minimum x** coordinate defined.
    #[inline]
    pub fn from_min_x(min_x: u16) -> Self {
        Self::new(Some(min_x), None, None, None)
    }

    /// Creates a new partial rectangle with only the **maximum x** coordinate defined.
    #[inline]
    pub fn from_max_x(max_x: u16) -> Self {
        Self::new(None, Some(max_x), None, None)
    }

    /// Creates a new partial rectangle with only the **minimum y** coordinate defined.
    #[inline]
    pub fn from_min_y(min_y: u16) -> Self {
        Self::new(None, None, Some(min_y), None)
    }

    /// Creates a new partial rectangle with only the **maximum y** coordinate defined.
    #[inline]
    pub fn from_max_y(max_y: u16) -> Self {
        Self::new(None, None, None, Some(max_y))
    }

    /// Returns a new partial rectangle with the new **minimum x** coordinate.
    #[inline]
    #[must_use]
    pub fn with_min_x(mut self, min_x: u16) -> Self {
        self.min_x = Some(min_x);
        self
    }

    /// Returns a new partial rectangle with the new **maximum x** coordinate.
    #[inline]
    #[must_use]
    pub fn with_max_x(mut self, max_x: u16) -> Self {
        self.max_x = Some(max_x);
        self
    }

    /// Returns a new partial rectangle with the new **minimum y** coordinate.
    #[inline]
    #[must_use]
    pub fn with_min_y(mut self, min_y: u16) -> Self {
        self.min_y = Some(min_y);
        self
    }

    /// Returns a new partial rectangle with the new **maximum y** coordinate.
    #[inline]
    #[must_use]
    pub fn with_max_y(mut self, max_y: u16) -> Self {
        self.max_y = Some(max_y);
        self
    }

    /// Checks if a point is inside the partial rectangle.
    #[inline]
    pub fn contains(&self, x: u16, y: u16) -> bool {
        self.min_x.map_or(true, |min_x| x >= min_x)
            && self.max_x.map_or(true, |max_x| x < max_x)
            && self.min_y.map_or(true, |min_y| y >= min_y)
            && self.max_y.map_or(true, |max_y| y < max_y)
    }
}
