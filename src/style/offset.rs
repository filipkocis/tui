#[derive(Debug, Clone, Copy)]
pub enum Offset {
    /// Absolute offset in screen space
    Absolute(i16, i16),
    /// Relative offset from parent, node is positioned absolutely.
    AbsolutelyRelative(i16, i16),
    /// Relative offset from parent, node is positioned inside the parent
    Translate(i16, i16),
}

impl Offset {
    #[inline(always)]
    pub fn x(&self) -> i16 {
        match self {
            Self::Absolute(x, _) => *x,
            Self::AbsolutelyRelative(x, _) => *x,
            Self::Translate(x, _) => *x,
        }
    }

    #[inline(always)]
    pub fn y(&self) -> i16 {
        match self {
            Self::Absolute(_, y) => *y,
            Self::AbsolutelyRelative(_, y) => *y,
            Self::Translate(_, y) => *y,
        }
    }

    /// Returns wheter the node should be absolutely positioned
    #[inline(always)]
    pub fn is_absolute(&self) -> bool {
        match self {
            Self::Absolute(..) | Self::AbsolutelyRelative(..) => true,
            _ => false,
        }
    }

    /// True if self is the `AbsolutelyRelative` variant
    #[inline(always)]
    pub fn is_absolutely_relative(&self) -> bool {
        match self {
            Self::AbsolutelyRelative(..) => true,
            _ => false,
        }
    }

    /// True if self is the `Translate` variant
    #[inline(always)]
    pub fn is_translate(&self) -> bool {
        matches!(self, Self::Translate(..))
    }

    #[inline(always)]
    pub fn tuple(self) -> (i16, i16) {
        match self {
            Self::Absolute(x, y) => (x, y),
            Self::AbsolutelyRelative(x, y) => (x, y),
            Self::Translate(x, y) => (x, y),
        }
    }

    #[inline(always)]
    pub fn add(self, child: Self) -> Self {
        match child {
            Self::Translate(x, y) | Self::AbsolutelyRelative(x, y) => {
                Self::Translate(self.x() + x, self.y() + y)
            }
            Self::Absolute(..) => child,
        }
    }

    #[inline(always)]
    pub fn add_tuple(self, tuple: (i16, i16)) -> Self {
        self.add(Self::Translate(tuple.0, tuple.1))
    }

    /// Unlike `add`, this method does not change the variant of the offset.
    #[inline(always)]
    pub fn add_without_variant_change(self, tuple: (i16, i16)) -> Self {
        let (nx, ny) = tuple;
        match self {
            Self::Translate(x, y) => Self::Translate(x + nx, y + ny),
            Self::AbsolutelyRelative(x, y) => Self::AbsolutelyRelative(x + nx, y + ny),
            Self::Absolute(x, y) => Self::Absolute(x + nx, y + ny),
        }
    }
}

impl Default for Offset {
    fn default() -> Self {
        Offset::Translate(0, 0)
    }
}
