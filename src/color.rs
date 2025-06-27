use crossterm::style::Color;

pub fn srgb_to_linear(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    fn convert(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    (convert(r), convert(g), convert(b))
}

pub fn linear_to_srgb(r: f64, g: f64, b: f64) -> (u8, u8, u8) {
    fn convert(c: f64) -> u8 {
        let c = if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (c.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    (convert(r), convert(g), convert(b))
}

pub fn linear_rgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;
    (l, a, b)
}

pub fn oklab_to_linear_rgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_.powi(3);
    let m = m_.powi(3);
    let s = s_.powi(3);

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = 0.0415550574 * l - 0.7037494038 * m + 1.6626131094 * s;

    (r, g, b)
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
/// OKLCH color format representation.
pub struct Oklch {
    /// Lightness [0.0 – 1.0]
    pub l: f64,
    /// Chroma (color intensity)
    pub c: f64,
    /// Hue angle in degrees [0.0 – 360.0)
    pub h: f64,
}

impl Oklch {
    /// Create a new OKLCH color.
    pub fn new(l: f64, c: f64, h: f64) -> Self {
        Self { l, c, h }
    }

    /// Convert sRGB (0–255) to OKLCH
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let (r_lin, g_lin, b_lin) = srgb_to_linear(r, g, b);
        let (l, a, b) = linear_rgb_to_oklab(r_lin, g_lin, b_lin);
        let c = (a * a + b * b).sqrt();
        let h = b.atan2(a).to_degrees().rem_euclid(360.0);
        Self { l, c, h }
    }

    /// Convert OKLCH to sRGB (0–255)
    pub fn to_rgb(self) -> (u8, u8, u8) {
        let a = self.c * self.h.to_radians().cos();
        let b = self.c * self.h.to_radians().sin();
        let (r_lin, g_lin, b_lin) = oklab_to_linear_rgb(self.l, a, b);
        linear_to_srgb(r_lin, g_lin, b_lin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
/// HSL color format representation.
pub struct Hsl {
    /// Hue angle in degrees [0.0 - 360.0)
    pub h: f64,
    /// Saturation [0.0 - 1.0]
    pub s: f64,
    /// Lightness [0.0 - 1.0]
    pub l: f64,
}

impl Hsl {
    pub fn new(h: f64, s: f64, l: f64) -> Self {
        Self { h, s, l }
    }

    pub fn to_rgb(self) -> (u8, u8, u8) {
        let Hsl { h, s, l } = self;

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let h_prime = h / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());

        let (r1, g1, b1) = match h_prime as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            5 | _ => (c, 0.0, x),
        };

        let m = l - c / 2.0;
        let (r, g, b) = (
            ((r1 + m) * 255.0).round() as u8,
            ((g1 + m) * 255.0).round() as u8,
            ((b1 + m) * 255.0).round() as u8,
        );

        (r, g, b)
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        // Lightness
        let l = (max + min) / 2.0;

        // Saturation
        let s = if delta == 0.0 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };

        // Hue
        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        // Normalize hue to [0, 360)
        let h = if h < 0.0 { h + 360.0 } else { h };

        Self { h, s, l }
    }
}

impl From<Color> for Hsl {
    fn from(value: Color) -> Self {
        match value {
            Color::Rgb { r, g, b } => Hsl::from_rgb(r, g, b),
            _ => Hsl::new(0.0, 0.0, 0.0),
        }
    }
}

impl From<Hsl> for Color {
    fn from(value: Hsl) -> Self {
        let (r, g, b) = value.to_rgb();
        Color::Rgb { r, g, b }
    }
}

impl From<Color> for Oklch {
    fn from(value: Color) -> Self {
        match value {
            Color::Rgb { r, g, b } => Oklch::from_rgb(r, g, b),
            _ => Oklch::new(0.0, 0.0, 0.0),
        }
    }
}

impl From<Oklch> for Color {
    fn from(value: Oklch) -> Self {
        let (r, g, b) = value.to_rgb();
        Color::Rgb { r, g, b }
    }
}
