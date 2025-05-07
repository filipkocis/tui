#[derive(Debug, Clone, Copy)]
pub enum Offset {
    Absolute(i16, i16),
    Translate(i16, i16),
}

impl Offset {
    #[inline(always)]
    pub fn x(&self) -> i16 {
        match self {
            Self::Absolute(x, _) => *x,
            Self::Translate(x, _) => *x,
        }
    }

    #[inline(always)]
    pub fn y(&self) -> i16 {
        match self {
            Self::Absolute(_, y) => *y,
            Self::Translate(_, y) => *y,
        }
    }

    #[inline(always)]
    pub fn is_absolute(&self) -> bool {
        matches!(self, Self::Absolute(..))
    }

    #[inline(always)]
    pub fn is_translate(&self) -> bool {
        matches!(self, Self::Translate(..))
    }

    #[inline(always)]
    pub fn tuple(self) -> (i16, i16) {
        match self {
            Self::Absolute(x, y) => (x, y),
            Self::Translate(x, y) => (x, y),
        }
    }

    #[inline(always)]
    pub fn add(self, child: Self) -> Self {
        match (self, child) {
            (Self::Translate(x1, y1), Self::Translate(x2, y2)) => Self::Translate(x1 + x2, y1 + y2),
            (Self::Absolute(x1, y1), Self::Translate(x2, y2)) => Self::Translate(x1 + x2, y1 + y2),
            (_, Self::Absolute(..)) => child,
        }
    }

    #[inline(always)]
    pub fn add_tuple(self, tuple: (i16, i16)) -> Self {
        self.add(Self::Translate(tuple.0, tuple.1))
    }
}

impl Default for Offset {
    fn default() -> Self {
        Offset::Translate(0, 0)
    }
}
