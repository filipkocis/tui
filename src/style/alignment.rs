/// Justify content in the direction of the flex container
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Justify {
    /// Start of the container
    #[default]
    Start,
    /// Center of the container
    Center,
    /// End of the container
    End,
    /// Space between items
    SpaceBetween,
    /// Space around items
    SpaceAround,
}

impl Justify {
    /// True if self is [Justify::Start] variant
    #[inline(always)]
    pub fn is_start(self) -> bool {
        matches!(self, Justify::Start)
    }

    /// True if self is [Justify::Center] variant
    #[inline(always)]
    pub fn is_center(self) -> bool {
        matches!(self, Justify::Center)
    }

    /// True if self is [Justify::End] variant
    #[inline(always)]
    pub fn is_end(self) -> bool {
        matches!(self, Justify::End)
    }

    /// True if self is [Justify::SpaceBetween] variant
    #[inline(always)]
    pub fn is_space_between(self) -> bool {
        matches!(self, Justify::SpaceBetween)
    }

    /// True if self is [Justify::SpaceAround] variant
    #[inline(always)]
    pub fn is_space_around(self) -> bool {
        matches!(self, Justify::SpaceAround)
    }

    /// True if self is [Justify::SpaceBetween] or [Justify::SpaceAround] variant
    #[inline(always)]
    pub fn is_spaced(self) -> bool {
        self.is_space_between() || self.is_space_around()
    }
}
