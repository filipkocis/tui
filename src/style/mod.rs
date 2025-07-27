mod offset;
mod padding;
mod size;
mod alignment;

pub use offset::Offset;
pub use padding::Padding;
pub use size::{Size, SizeValue};
pub use alignment::{Justify, Align};

use crossterm::style::Color;

use crate::text::Text;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub offset: Offset,
    /// Stacking order among siblings
    pub z_index: i16,

    pub size: Size,
    pub min_size: Size,
    pub max_size: Size,

    pub fg: Option<Color>,
    pub bg: Option<Color>,

    pub bold: bool,
    pub underline: bool,
    pub dim: bool,
    pub crossed: bool,

    pub padding: Padding,
    pub border: (bool, bool, bool, bool, Option<Color>),

    pub justify: Justify,
    pub align: Align,
    pub flex_row: bool,
    // pub grow: bool,
    pub gap: (u16, u16),
}

impl Style {
    pub fn apply(&self, other: &Style) -> Style {
        other.clone()
    }

    /// Returns `size.max(min).min(max)` for the width
    pub fn clamped_width(&self) -> u16 {
        self.size
            .width
            .computed_size()
            .min(self.max_size.width.computed_size())
            .max(self.min_size.width.computed_size())
    }

    /// Returns `size.max(min).min(max)` for the height
    pub fn clamped_height(&self) -> u16 {
        self.size
            .height
            .computed_size()
            .min(self.max_size.height.computed_size())
            .max(self.min_size.height.computed_size())
    }

    const DEFAULT_MIN_SIZE: (u16, u16) = (0, 0);
    const DEFAULT_MAX_SIZE: (u16, u16) = (u16::MAX, u16::MAX);

    /// Calculates percentage sizes, applies clamping, and calculates wrapped text height (if auto)
    pub fn compute_percentage_size(&mut self, parent_size: Size, text: &mut Text) {
        // Calc min max
        self.min_size = self
            .min_size
            .compute_size(parent_size, Self::DEFAULT_MIN_SIZE);
        self.max_size = self
            .max_size
            .compute_size(parent_size, Self::DEFAULT_MAX_SIZE);

        // Calc size
        let Size {
            mut width,
            mut height,
        } = self.size.compute_size(parent_size, self.size.tuple());

        // subtract padding and borders from percentages
        if width.is_percent() {
            let size = width.computed_size();
            let new_size = size.saturating_sub(self.extra_width());
            width = width.set_computed_size(new_size);
        }
        if height.is_percent() {
            let size = height.computed_size();
            let new_size = size.saturating_sub(self.extra_height());
            height = height.set_computed_size(new_size);
        }

        // Clamp size
        self.size = Size::new(width, height).clamp_computed_size(self.min_size, self.max_size);

        // Finalize and wrap text
        let self_width = self.size.width.computed_size();
        let line_wrap_len_diff = text.wrap_text(self_width);

        // Recalculate height with wrapped text height, if height is auto
        if self.size.height.is_auto() {
            let text_height_diff = line_wrap_len_diff.min(u16::MAX as usize) as u16;

            // Add text height wrap difference to the current size
            let height = self.size.height.computed_size();
            self.size.height = self
                .size
                .height
                .set_computed_size(height + text_height_diff);

            // Clamp size
            self.size.height = self
                .size
                .height
                .clamp_computed_size(self.min_size.height, self.max_size.height);
        }
    }

    /// Calculates auto (for text) and intrinsic sizes
    pub fn compute_intrinsic_size(&mut self, text: &Text) {
        // Default auto parent since we only care about intrinsic size
        let parent_size = Size::default();

        // Calc min max
        self.min_size = self
            .min_size
            .compute_size(parent_size, Self::DEFAULT_MIN_SIZE);
        self.max_size = self
            .max_size
            .compute_size(parent_size, Self::DEFAULT_MAX_SIZE);

        // Calc size
        let Size {
            mut width,
            mut height,
        } = self.size.compute_size(parent_size, (0, 0));

        // Get text dimensions
        let (text_width, text_height) = text.get_visual_size();

        // Set to intrinsic text size if auto size
        if width.is_auto() {
            width = width.set_computed_size(text_width);
        }
        if height.is_auto() {
            height = height.set_computed_size(text_height);
        }

        // Set size
        self.size = Size::new(width, height);
    }

    /// Returns the extra width (horizontal padding and borders)
    #[inline]
    pub fn extra_width(&self) -> u16 {
        self.padding
            .horizontal()
            .saturating_add(self.border.2 as u16)
            .saturating_add(self.border.3 as u16)
    }

    /// Returns the extra height (vertical padding and borders)
    #[inline]
    pub fn extra_height(&self) -> u16 {
        self.padding
            .vertical()
            .saturating_add(self.border.0 as u16)
            .saturating_add(self.border.1 as u16)
    }

    /// Total computed width
    pub fn total_width(&self) -> u16 {
        self.clamped_width().saturating_add(self.extra_width())
    }

    /// Total computed height
    pub fn total_height(&self) -> u16 {
        self.clamped_height().saturating_add(self.extra_height())
    }

    /// Total computed unclamped width
    pub fn total_width_unclamped(&self) -> u16 {
        self.size
            .width
            .computed_size()
            .saturating_add(self.extra_width())
    }

    /// Total computed unclamped height
    pub fn total_height_unclamped(&self) -> u16 {
        self.size
            .height
            .computed_size()
            .saturating_add(self.extra_height())
    }

    /// Total computed width and height
    #[inline]
    pub fn total_size(&self) -> (u16, u16) {
        (self.total_width(), self.total_height())
    }
}

impl Size {
    /// Computes the size based on parent's size, if self is auto, default (width, height) is used.
    pub fn compute_size(self, parent: Self, default: (u16, u16)) -> Self {
        let width = self.width.compute_size(parent.width, default.0);
        let height = self.height.compute_size(parent.height, default.1);
        Self::new(width, height)
    }

    /// Clamp computed size between min and max
    pub fn clamp_computed_size(self, min: Self, max: Self) -> Self {
        let width = self.width.clamp_computed_size(min.width, max.width);
        let height = self.height.clamp_computed_size(min.height, max.height);
        Self::new(width, height)
    }
}

impl SizeValue {
    /// Computes the size based on parent's size, if self is auto, default is used.
    pub fn compute_size(self, parent: Self, default: u16) -> Self {
        match self {
            Self::Auto(_) => self.set_computed_size(default),
            Self::Cells(cells, _) => self.set_computed_size(cells),
            Self::Percent(p, _) => {
                let size = parent.computed_size();
                let size = (size as f32 * p as f32 / 100.0).floor() as u16;
                self.set_computed_size(size)
            }
        }
    }

    /// Clamp computed size between min and max
    pub fn clamp_computed_size(self, min: Self, max: Self) -> Self {
        let size = self.computed_size();
        let min = min.computed_size();
        let max = max.computed_size();

        let clamped = size.min(max).max(min);
        self.set_computed_size(clamped)
    }
}
