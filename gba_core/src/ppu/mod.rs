use crate::{HEIGHT, WIDTH};

pub struct Ppu {
    /// Framebuffer: row major, each pixel is ARGB, length (WIDTH * HEIGHT).
    pub framebuffer: Box<[u32]>,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            framebuffer: vec![0xFFFF7518u32; WIDTH * HEIGHT].into_boxed_slice(),
        }
    }
}
