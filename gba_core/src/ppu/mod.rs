use crate::{HEIGHT, WIDTH};
use registers::*;

mod registers;

pub struct Ppu {
    /// Framebuffer: row major, each pixel is ARGB, length (WIDTH * HEIGHT).
    pub framebuffer: Box<[u32]>,

    /// Register DISPCNT - LCD Control
    pub dispcnt: DisplayControl,

    /// Register DISPSTAT - General LCD Status,
    pub dispstat: DisplayStatus,

    /// Current scanline (0..=227). 160..=227 are in vblank.
    pub vcount: u16,

    /// VRAM - Video Ram - 96 KiB
    pub vram: Box<[u8]>,

    /// BG/OBJ Palette RAM - 1 KiB
    pub palette: Box<[u8]>,

    /// OAM - Object Attribute Memory - 1 KiB
    pub oam: Box<[u8]>,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            framebuffer: vec![0xFFFF7518u32; WIDTH * HEIGHT].into_boxed_slice(),
            dispcnt: DisplayControl::default(),
            dispstat: DisplayStatus::default(),
            vcount: 0,

            vram: vec![0; 96 * 1024].into_boxed_slice(),
            palette: vec![0; 1024].into_boxed_slice(),
            oam: vec![0; 1024].into_boxed_slice(),
        }
    }
}
