mod offset;
mod size;

pub use offset::Offset;
pub use size::{Size, SizeValue};

use crossterm::style::Color;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub offset: Offset,
    pub size: Size,

    pub fg: Option<Color>,
    pub bg: Option<Color>,

    pub bold: bool,
    pub underline: bool,
    pub dim: bool,
    pub crossed: bool,

    pub padding: (u16, u16, u16, u16),
    pub border: (bool, bool, bool, bool, Option<Color>),

    pub flex_row: bool,
    pub grow: bool,
    pub gap: (u16, u16),
}

impl Style {
    pub fn apply(&self, other: &Style) -> Style {
        other.clone()
    }
}
