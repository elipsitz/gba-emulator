#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Color15(pub u16);

impl Color15 {
    pub const TRANSPARENT: Color15 = Color15(0x8000);
    pub const WHITE: Color15 = Color15(0x7FFF);
    pub const BLACK: Color15 = Color15(0x0000);

    /// Convert the 15-bit color to 32-bit ARGB.
    pub fn as_argb(self) -> u32 {
        // Source: xbbbbbgggggrrrrr
        // Output: ARGB
        let r = (((self.0 >> 0) & 0b11111) as u32) << 19;
        let g = (((self.0 >> 5) & 0b11111) as u32) << 11;
        let b = (((self.0 >> 10) & 0b11111) as u32) << 3;
        0xFF00_0000 | r | g | b
    }

    pub fn as_rgb(self) -> (u16, u16, u16) {
        let r = (self.0 >> 0) & 0b11111;
        let g = (self.0 >> 5) & 0b11111;
        let b = (self.0 >> 10) & 0b11111;
        (r, g, b)
    }

    /// Convert three components to a color. r, g, b must all be < 32.
    pub fn from_rgb(r: u16, g: u16, b: u16) -> Color15 {
        Color15(r | (g << 5) | (b << 10))
    }

    /// Returns whether this color is transparent.
    pub fn transparent(self) -> bool {
        self == Color15::TRANSPARENT
    }

    pub fn blend(a: Color15, b: Color15, a_weight: u16, b_weight: u16) -> Color15 {
        let a_weight = a_weight.min(16);
        let b_weight = b_weight.min(16);
        let (r_a, g_a, b_a) = a.as_rgb();
        let (r_b, g_b, b_b) = b.as_rgb();
        let r = ((r_a * a_weight + r_b * b_weight) / 16).min(31);
        let g = ((g_a * a_weight + g_b * b_weight) / 16).min(31);
        let b = ((b_a * a_weight + b_b * b_weight) / 16).min(31);
        Color15::from_rgb(r, g, b)
    }
}
