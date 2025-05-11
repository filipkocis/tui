mod offset;
mod size;

pub use offset::Offset;
pub use size::{Size, SizeValue};

use crossterm::style::Color;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub offset: Offset,
    pub size: Size,
    pub min_size: Size,
    pub max_size: Size,

    pub fg: Option<Color>,
    pub bg: Option<Color>,

    pub bold: bool,
    pub underline: bool,
    pub dim: bool,
    pub crossed: bool,

    pub padding: (u16, u16, u16, u16),
    pub border: (bool, bool, bool, bool, Option<Color>),

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
            .width()
            .computed_size()
            .min(self.max_size.width().computed_size())
            .max(self.min_size.width().computed_size())
    }

    /// Returns `size.max(min).min(max)` for the height
    pub fn clamped_height(&self) -> u16 {
        self.size
            .height()
            .computed_size()
            .min(self.max_size.height().computed_size())
            .max(self.min_size.height().computed_size())
    }

    /// Computes `size, min, max` based on parent's size in a top-down manner.
    pub fn compute_size_td(&mut self, parent_size: Size) {
        self.min_size = self.min_size.compute_size(parent_size, 0);
        self.max_size = self.max_size.compute_size(parent_size, u16::MAX);

        self.size = self
            .size
            .compute_size(parent_size, u16::MAX)
            .clamp_computed_size(self.min_size, self.max_size)
    }
}

impl Size {
    /// Computes the size based on parent's size, if self is auto, default is used.
    pub fn compute_size(self, parent: Self, default: u16) -> Self {
        let width = self.width().compute_size(parent.width(), default);
        let height = self.height().compute_size(parent.height(), default);
        Self(width, height)
    }

    /// Clamp computed size between min and max
    pub fn clamp_computed_size(self, min: Self, max: Self) -> Self {
        let width = self.width().clamp_computed_size(min.width(), max.width());
        let height = self
            .height()
            .clamp_computed_size(min.height(), max.height());
        Self(width, height)
    }
}

impl SizeValue {
    /// Computes the size based on parent's size, if self is auto, default is used.
    pub fn compute_size(self, parent: Self, default: u16) -> Self {
        match self {
            Self::Auto(_) => self.set_computed_size(default),
            Self::Cells(..) => self,
            Self::Percent(p, _) => {
                if parent.is_auto() {
                    self.set_computed_size(0)
                } else {
                    let size = parent.computed_size();
                    let size = (size as f32 * p as f32 / 100.0).floor() as u16;
                    self.set_computed_size(size)
                }
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
