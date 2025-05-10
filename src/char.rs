use std::fmt::Display;

use crossterm::style::{Attribute, Color, SetBackgroundColor, SetForegroundColor};

#[derive(Debug, Clone)]
pub enum Char {
    Char(char),
    Code(Code),
}

impl Char {
    #[inline]
    pub fn is_code(&self) -> bool {
        matches!(self, Char::Code(_))
    }

    #[inline]
    pub fn is_char(&self) -> bool {
        matches!(self, Char::Char(_))
    }

    /// True if `self.is_code() && code.is_reset()`
    #[inline]
    pub fn is_reset_code(&self) -> bool {
        match self {
            Char::Code(code) => code.is_reset(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
