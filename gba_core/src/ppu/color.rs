#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Color15(pub u16);

impl Color15 {
    pub const TRANSPARENT: Color15 = Color15(0x8000);

    /// Convert the 15-bit color to ARGB.
    pub fn as_argb(self) -> u32 {
        // Source: xbbbbbgggggrrrrr
        // Output: ARGB
        let r = (((self.0 >> 0) & 0b11111) as u32) << 19;
        let g = (((self.0 >> 5) & 0b11111) as u32) << 11;
        let b = (((self.0 >> 10) & 0b11111) as u32) << 3;
        0xFF00_0000 | r | g | b
    }
}
