use crate::Style;

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
    /// Space evenly distributed
    SpaceEvenly,
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

    /// True if self is [Justify::SpaceEvenly] variant
    #[inline(always)]
    pub fn is_space_evenly(self) -> bool {
        matches!(self, Justify::SpaceEvenly)
    }

    /// True if self is any of the space variants
    #[inline(always)]
    pub fn is_spaced(self) -> bool {
        self.is_space_between() || self.is_space_around() || self.is_space_evenly()
    }

    /// Returns the offset for justifying an item within a container of given size.
    #[inline(always)]
    pub fn get_start_offset(
        self,
        free_content_size: i16,
        item_count: usize,
        flex_row: bool,
    ) -> (i16, i16) {
        let offset = match self {
            Self::Center => free_content_size / 2,
            Self::End => free_content_size,
            Self::SpaceAround => (free_content_size / item_count as i16 / 2).max(0),
            Self::SpaceEvenly => (free_content_size / (item_count + 1) as i16).max(0),
            _ => 0,
        };

        if flex_row { (offset, 0) } else { (0, offset) }
    }
}

/// Align items in the cross axis of the flex container
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    /// Start of the cross axis
    #[default]
    Start,
    /// Center of the cross axis
    Center,
    /// End of the cross axis
    End,
}

impl Align {
    /// True if self is [Align::Start] variant
    #[inline(always)]
    pub fn is_start(self) -> bool {
        matches!(self, Align::Start)
    }

    /// True if self is [Align::Center] variant
    #[inline(always)]
    pub fn is_center(self) -> bool {
        matches!(self, Align::Center)
    }

    /// True if self is [Align::End] variant
    #[inline(always)]
    pub fn is_end(self) -> bool {
        matches!(self, Align::End)
    }

    /// Returns the offset for aligning an item within a container of given size.
    #[inline(always)]
    pub fn alignment_offset(self, container_size: u16, item_size: u16) -> i32 {
        match self {
            Self::Start => 0,
            Self::Center => (container_size as i32 - item_size as i32) / 2,
            Self::End => container_size as i32 - item_size as i32,
        }
    }

    /// Returns the offset for aligning a child within a parent node.
    pub fn get_child_extra_offset(self, container: &Style, item: &Style) -> (i16, i16) {
        let (container_size, item_size) = if container.flex_row {
            (container.clamped_height(), item.total_height())
        } else {
            (container.clamped_height(), item.total_height())
        };

        let offset = self.alignment_offset(container_size, item_size);
        if container.flex_row {
            (0, offset as i16)
        } else {
            (offset as i16, 0)
        }
    }
}
