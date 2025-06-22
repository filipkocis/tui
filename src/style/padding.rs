/// Padding for node's style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Padding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Padding {
    /// Creates a new padding
    #[inline]
    pub fn new(top_bottom: u16, left_right: u16) -> Self {
        Self::separate(top_bottom, left_right, top_bottom, left_right)
    }

    /// Creates a new padding from separate values
    #[inline]
    pub fn separate(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates a new padding with identical values
    #[inline]
    pub fn all(padding: u16) -> Self {
        Self::new(padding, padding)
    }

    /// Creates a new padding with only top padding
    #[inline]
    pub fn top(top: u16) -> Self {
        Self::separate(top, 0, 0, 0)
    }

    /// Creates a new padding with only right padding
    #[inline]
    pub fn right(right: u16) -> Self {
        Self::separate(0, right, 0, 0)
    }

    /// Creates a new padding with only bottom padding
    #[inline]
    pub fn bottom(bottom: u16) -> Self {
        Self::separate(0, 0, bottom, 0)
    }

    /// Creates a new padding with only left padding
    #[inline]
    pub fn left(left: u16) -> Self {
        Self::separate(0, 0, 0, left)
    }

    /// Returns the horizontal padding
    #[inline]
    pub fn horizontal(&self) -> u16 {
        self.left + self.right
    }

    /// Returns the vertical padding
    #[inline]
    pub fn vertical(&self) -> u16 {
        self.top + self.bottom
    }

    /// Returns self as a tuple (top, right, bottom, left)
    #[inline]
    pub fn tuple(&self) -> (u16, u16, u16, u16) {
        self.into()
    }

    /// Returns self as a vec [top, right, bottom, left]
    #[inline]
    pub fn vec(&self) -> [u16; 4] {
        [self.top, self.right, self.bottom, self.left]
    }
}

impl Into<(u16, u16, u16, u16)> for &Padding {
    fn into(self) -> (u16, u16, u16, u16) {
        (self.top, self.right, self.bottom, self.left)
    }
}

impl Into<Padding> for (u16, u16, u16, u16) {
    fn into(self) -> Padding {
        Padding::separate(self.0, self.1, self.2, self.3)
    }
}

impl Into<Padding> for (u16, u16, u16) {
    fn into(self) -> Padding {
        Padding::separate(self.0, self.1, self.2, self.1)
    }
}

impl Into<Padding> for (u16, u16) {
    fn into(self) -> Padding {
        Padding::new(self.0, self.1)
    }
}

impl Into<Padding> for u16 {
    fn into(self) -> Padding {
        Padding::all(self)
    }
}
