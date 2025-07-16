use std::fmt::Display;

use crossterm::style::{Attribute, Color, SetBackgroundColor, SetForegroundColor};

use crate::text::Attrs;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum Code {
    Attribute(Attribute),
    Background(Color),
    Foreground(Color),
}

impl Code {
    /// Returns the reset variant of the code
    pub fn into_reset(&self) -> Self {
        match self {
            Self::Attribute(_) => Self::Attribute(Attribute::Reset),
            Self::Background(_) => Self::Background(Color::Reset),
            Self::Foreground(_) => Self::Foreground(Color::Reset),
        }
    }

    /// True if the code is a reset code
    pub fn is_reset(&self) -> bool {
        match self {
            Self::Attribute(attr) => *attr == Attribute::Reset,
            Self::Background(color) => *color == Color::Reset,
            Self::Foreground(color) => *color == Color::Reset,
        }
    }

    /// True if the code is an attribute code
    pub fn is_attribute(&self) -> bool {
        matches!(self, Self::Attribute(_))
    }
}

impl Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Code::Attribute(attr) => write!(f, "{}", attr),
            Code::Background(color) => write!(f, "{}", SetBackgroundColor(*color)),
            Code::Foreground(color) => write!(f, "{}", SetForegroundColor(*color)),
        }
    }
}

#[derive(Default)]
pub struct CodeUnit {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub attrs: Attrs,
}
