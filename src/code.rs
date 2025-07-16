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

#[derive(Default, Debug, Clone, Copy, Hash, PartialEq, Eq)]
/// Represents a single code unit for one character in the terminal. Contains its foreground and
/// background colors, as well as any attributes that apply to it.
pub struct CodeUnit {
    /// The foreground color of the code unit, `None` if not set or if `reset`.
    fg: Option<Color>,
    /// The background color of the code unit, `None` if not set or if `reset`.
    bg: Option<Color>,
    /// The attributes of the code unit, `Attrs(0)` if not set or if `reset`.
    attrs: Attrs,
}

impl CodeUnit {
    /// Creates a new `CodeUnit` with no codes set.
    #[inline]
    pub fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            attrs: Attrs::default(),
        }
    }

    /// Returns true if the code unit has no codes set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.fg.is_none() && self.bg.is_none() && self.attrs.is_empty()
    }

    /// Returns the foreground color.
    #[inline]
    pub fn fg(&self) -> Option<Color> {
        self.fg
    }

    /// Sets the foreground color, `None` if `reset`.
    #[inline]
    pub fn set_fg(&mut self, color: Color) {
        if color == Color::Reset {
            self.fg = None;
        } else {
            self.bg = Some(color);
        }
    }

    /// Returns the background color.
    #[inline]
    pub fn bg(&self) -> Option<Color> {
        self.bg
    }

    /// Sets the background color, `None` if `reset`.
    #[inline]
    pub fn set_bg(&mut self, color: Color) {
        if color == Color::Reset {
            self.bg = None;
        } else {
            self.bg = Some(color);
        }
    }

    /// Returns the attributes.
    #[inline]
    pub fn attrs(&self) -> Attrs {
        self.attrs
    }

    /// Sets a new attribute.
    #[inline]
    pub fn apply_attr(&mut self, attr: Attribute) {
        self.attrs = self.attrs.apply(attr);
    }
}
