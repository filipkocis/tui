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

    /// Applies the given attrivute to the current attributes and returns a new `Attrs` bitfield.
    pub fn apply(self, attr: Attribute) -> Self {
        /// Sets the bit at position `n` in `num`.
        #[inline(always)]
        fn set(num: u16, n: u16) -> u16 {
            num | (1 << n)
        }

        /// Unsets the bit at position `n` in `num`.
        #[inline(always)]
        fn unset(num: u16, n: u16) -> u16 {
            num & !(1 << n)
        }

        /// Unsets all underline bits.
        fn unset_underline(num: u16) -> u16 {
            let ul = unset(num, 3);
            let du = unset(ul, 4);
            let uc = unset(du, 5);
            let ud = unset(uc, 6);
            unset(ud, 7)
        }

        let n = self.0;
        let new = match attr {
            Attribute::Reset => 0,

            Attribute::Bold => set(n, 0),
            Attribute::NoBold => unset(n, 0),

            Attribute::Italic => set(n, 1),
            Attribute::NoItalic => unset(n, 1),

            Attribute::Dim => set(n, 2),
            Attribute::NormalIntensity => unset(unset(unset(n, 0), 1), 2),

            Attribute::Underlined => set(unset_underline(n), 3),
            Attribute::DoubleUnderlined => set(unset_underline(n), 4),
            Attribute::Undercurled => set(unset_underline(n), 5),
            Attribute::Underdotted => set(unset_underline(n), 6),
            Attribute::Underdashed => set(unset_underline(n), 7),
            Attribute::NoUnderline => unset_underline(n),

            Attribute::SlowBlink => set(n, 8),
            Attribute::RapidBlink => set(n, 9),
            Attribute::NoBlink => 25,

            Attribute::Reverse => set(n, 10),
            Attribute::NoReverse => unset(n, 10),

            Attribute::Hidden => set(n, 11),
            Attribute::NoHidden => unset(n, 11),

            Attribute::CrossedOut => set(n, 12),
            Attribute::NotCrossedOut => unset(n, 12),

            Attribute::OverLined => set(n, 13),
            Attribute::NotOverLined => unset(n, 13),

            // Ignored
            Attribute::Fraktur
            | Attribute::Framed
            | Attribute::Encircled
            | Attribute::NotFramedOrEncircled => n,
            // Any new unsupported attrs
            _ => n,
        };

        Self(new)
    }
}
