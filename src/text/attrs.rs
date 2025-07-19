use crossterm::style::Attribute;

#[derive(Default, Debug, Hash, Clone, Copy, PartialEq, Eq)]
/// A bitfield representing the [`attributes`](Attribute) of a character.
pub struct Attrs(u16);

impl Attrs {
    /// Supported attribute flags (in order).
    pub const ATTRS: [Attribute; 14] = [
        Attribute::Bold,
        Attribute::Dim,
        Attribute::Italic,
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
            .map(|i| self.get(i as u16))
            .collect()
    }

    /// Returns the attribute at position `n` in the bitfield, if it is set.
    #[inline(always)]
    pub fn get_bit(self, n: u16) -> bool {
        n < Self::ATTRS.len() as u16 && (self.0 & (1 << n)) != 0
    }

    /// Returns the attribute at position `n` in the bitfield, if it is set.
    #[inline(always)]
    pub fn get(self, n: u16) -> Option<Attribute> {
        self.get_bit(n).then(|| Self::ATTRS[n as usize])
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

    /// Unsets all underlined attributes.
    pub fn unset_underlined(self) -> Attrs {
        self.unset(3) // underlined
            .unset(4) // double underlined
            .unset(5) // undercurled
            .unset(6) // underdotted
            .unset(7) // underdashed
    }

    /// Applies the given attribute to the current attributes and returns a new `Attrs` bitfield.
    pub fn apply(self, attr: Attribute) -> Self {
        match attr {
            Attribute::Reset => Self::default(),

            Attribute::Bold => self.unset(1).set(0),
            Attribute::NoBold => self.unset(0),
            Attribute::Dim => self.unset(0).set(1),
            Attribute::NormalIntensity => self.unset(0).unset(1),

            Attribute::Italic => self.set(2),
            Attribute::NoItalic => self.unset(2),

            Attribute::Underlined => self.unset_underlined().set(3),
            Attribute::DoubleUnderlined => self.unset_underlined().set(4),
            Attribute::Undercurled => self.unset_underlined().set(5),
            Attribute::Underdotted => self.unset_underlined().set(6),
            Attribute::Underdashed => self.unset_underlined().set(7),
            Attribute::NoUnderline => self.unset_underlined(),

            Attribute::SlowBlink => self.unset(9).set(8),
            Attribute::RapidBlink => self.unset(8).set(9),
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

    /// Returns the reset attribute for the given attribute.
    /// # Panics
    /// If the attribute cannot bve reset or is not supported. See [Attrs::ATTRS].
    #[inline]
    pub fn get_reset_attr(attr: Attribute) -> Attribute {
        let reset_attr = match attr {
            Attribute::Bold | Attribute::Dim => Attribute::NormalIntensity,

            Attribute::Italic => Attribute::NoItalic,

            Attribute::Underlined
            | Attribute::DoubleUnderlined
            | Attribute::Undercurled
            | Attribute::Underdotted
            | Attribute::Underdashed => Attribute::NoUnderline,

            Attribute::SlowBlink | Attribute::RapidBlink => Attribute::NoBlink,

            Attribute::Reverse => Attribute::NoReverse,
            Attribute::Hidden => Attribute::NoHidden,
            Attribute::CrossedOut => Attribute::NotCrossedOut,
            Attribute::OverLined => Attribute::NotOverLined,

            _ => panic!("Should not happen, got {attr:?}"),
        };

        reset_attr
    }

    /// Returns the reset codes for this attribute bitfield. May also return **set** codes if the
    /// reset codes apply to multiple but not all fields from `other` bitfield.
    /// Additional **set** codes are returned if the `other` bitfield has attributes that are not in
    /// `self`.
    pub fn into_change_codes(self, other: Self) -> Vec<Attribute> {
        let mut reset_codes = Vec::new();
        let mut set_codes = Vec::new();

        // Check each attribute in the current bitfield
        for (i, &attr) in Self::ATTRS.iter().enumerate() {
            let self_bit = self.0 & (1 << i) != 0;
            let other_bit = other.0 & (1 << i) != 0;

            if self_bit {
                // If the attribute is set in `self` but not in `other`, add its reset code
                if !other_bit {
                    reset_codes.push(Self::get_reset_attr(attr));
                }
            } else if other_bit {
                // If the attribute is set in `other` but not in `self`, add it as a set code
                set_codes.push(attr);
            }
        }

        // Combine reset and set codes
        reset_codes.extend(set_codes);
        reset_codes
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
