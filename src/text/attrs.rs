use crossterm::style::Attribute;

#[derive(Default, Debug, Hash, Clone, Copy, PartialEq, Eq)]
/// A bitfield representing the [`attributes`](Attribute) of a character.
pub struct Attrs(u16);

impl Attrs {
    /// Supported attribute flags (in order).
    pub const ATTRS: [Attribute; 14] = [
        Attribute::Bold,
        Attribute::Italic,
        Attribute::Dim,
        Attribute::Underlined,
        Attribute::DoubleUnderlined,
        Attribute::Undercurled,
        Attribute::Underdotted,
        Attribute::Underdashed,
        Attribute::SlowBlink,
        Attribute::RapidBlink,
        Attribute::Reverse,
        Attribute::Hidden,
        Attribute::CrossedOut,
        Attribute::OverLined,
    ];

    /// Returns true if the attributes are empty.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns self extracted as [Self::ATTRS] wrapped in `Option`.
    pub fn extract(self) -> Vec<Option<Attribute>> {
        (0..Self::ATTRS.len())
            .into_iter()
            .map(|i| {
                if self.0 & (1 << i) != 0 {
                    Some(Self::ATTRS[i])
                } else {
                    None
                }
            })
            .collect()
    }

    /// Sets the bit at position `n` in the bitfield.
    #[inline(always)]
    pub fn set(self, n: u16) -> Self {
        Self(self.0 | (1 << n))
    }

    /// Unsets the bit at position `n` in the bitfield.
    #[inline(always)]
    pub fn unset(self, n: u16) -> Self {
        Self(self.0 & !(1 << n))
    }

    /// Unsets all underline attributes.
    pub fn unset_underline(self) -> Attrs {
        self.unset(3) // underline
            .unset(4) // double underline
            .unset(5) // undercurled
            .unset(6) // underdotted
            .unset(7) // underdashed
    }

    /// Applies the given attribute to the current attributes and returns a new `Attrs` bitfield.
    pub fn apply(self, attr: Attribute) -> Self {
        match attr {
            Attribute::Reset => Self::default(),

            Attribute::Bold => self.set(0),
            Attribute::NoBold => self.unset(0),

            Attribute::Italic => self.set(1),
            Attribute::NoItalic => self.unset(1),

            Attribute::Dim => self.set(2),
            Attribute::NormalIntensity => self.unset(0).unset(1).unset(2),

            Attribute::Underlined => self.unset_underline().set(3),
            Attribute::DoubleUnderlined => self.unset_underline().set(4),
            Attribute::Undercurled => self.unset_underline().set(5),
            Attribute::Underdotted => self.unset_underline().set(6),
            Attribute::Underdashed => self.unset_underline().set(7),

            Attribute::SlowBlink => self.set(8),
            Attribute::RapidBlink => self.set(9),
            Attribute::NoBlink => self.unset(8).unset(9),

            Attribute::Reverse => self.set(10),
            Attribute::NoReverse => self.unset(10),

            Attribute::Hidden => self.set(11),
            Attribute::NoHidden => self.unset(11),

            Attribute::CrossedOut => self.set(12),
            Attribute::NotCrossedOut => self.unset(12),

            Attribute::OverLined => self.set(13),
            Attribute::NotOverLined => self.unset(13),

            // Ignored
            Attribute::Fraktur
            | Attribute::Framed
            | Attribute::Encircled
            | Attribute::NotFramedOrEncircled => self,
            // Any new unsupported attrs
            _ => self,
        }
    }
}

impl Iterator for Attrs {
    type Item = Attribute;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            return None;
        }

        for i in 0..Self::ATTRS.len() {
            if self.0 & (1 << i) != 0 {
                let attr = Self::ATTRS[i];
                self.0 &= !(1 << i); // Unset the bit
                return Some(attr);
            }
        }

        None
    }
}
