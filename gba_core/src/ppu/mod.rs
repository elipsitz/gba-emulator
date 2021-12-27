use crate::{
    mem::Memory,
    scheduler::{Event, PpuEvent},
    Gba, HEIGHT, WIDTH,
};
use registers::*;

mod registers;

#[allow(unused)]
mod constants {
    pub const PIXELS_WIDTH: usize = crate::WIDTH;
    pub const PIXELS_HEIGHT: usize = crate::HEIGHT;
    pub const PIXELS_HBLANK: usize = 68;
    pub const SCANLINES_VBLANK: usize = 68;
    pub const CYCLES_PIXEL: usize = 4;
    pub const CYCLES_HDRAW: usize = PIXELS_WIDTH * CYCLES_PIXEL;
    pub const CYCLES_HBLANK: usize = PIXELS_HBLANK * CYCLES_PIXEL;
    pub const CYCLES_SCANLINE: usize = CYCLES_HDRAW + CYCLES_HBLANK;
    pub const CYCLES_VDRAW: usize = CYCLES_SCANLINE * PIXELS_HEIGHT;
    pub const CYCLES_VBLANK: usize = CYCLES_SCANLINE * SCANLINES_VBLANK;
    pub const CYCLES_FRAME: usize = CYCLES_VDRAW + CYCLES_VBLANK;
}
pub use constants::*;

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

    /// Current frame.
    #[allow(unused)]
    pub frame: usize,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            framebuffer: vec![0xFFFF7518u32; WIDTH * HEIGHT].into_boxed_slice(),
            dispcnt: DisplayControl::default(),
            dispstat: DisplayStatus::default(),
            vcount: 0,
            frame: 0,

            // 96KiB, but we'll make it 128KiB for accesses
            vram: vec![0; 128 * 1024].into_boxed_slice(),
            palette: vec![0; 1024].into_boxed_slice(),
            oam: vec![0; 1024].into_boxed_slice(),
        }
    }
}

fn pixel16_to_32(pixel: u16) -> u32 {
    // Source: xbbbbbgggggrrrrr
    // Output: ARGB
    let r = (((pixel >> 0) & 0b11111) as u32) << 19;
    let g = (((pixel >> 5) & 0b11111) as u32) << 11;
    let b = (((pixel >> 10) & 0b11111) as u32) << 3;
    0xFF00_0000 | r | g | b
}

impl Gba {
    pub fn ppu_init(&mut self) {
        self.scheduler
            .push_event(Event::Ppu(PpuEvent::EndScanline), CYCLES_SCANLINE);
    }

    pub fn ppu_on_event(&mut self, event: PpuEvent, lateness: usize) {
        let (next_event, deadline) = match event {
            PpuEvent::EndScanline => self.ppu_draw_scanline(),
        };
        let deadline = deadline - lateness;
        self.scheduler.push_event(Event::Ppu(next_event), deadline);
    }

    fn ppu_draw_scanline(&mut self) -> (PpuEvent, usize) {
        match self.ppu.dispcnt.mode {
            0 => {}
            3 => {
                // Mode 3: 240x160, 16 bpp
                let line = self.ppu.vcount as usize;
                if line < PIXELS_HEIGHT {
                    let input = &mut self.ppu.vram[(PIXELS_WIDTH * line * 2)..];
                    let output = &mut self.ppu.framebuffer[(PIXELS_WIDTH * line)..];
                    for x in 0..PIXELS_WIDTH {
                        let color_15bit = input.read_16((x * 2) as u32);
                        output[x] = pixel16_to_32(color_15bit);
                    }
                }
            }
            4 => {
                // Mode 4: 240x160, 8 bpp (palette)
                let line = self.ppu.vcount as usize;
                if line < PIXELS_HEIGHT {
                    let input = &self.ppu.vram[(PIXELS_WIDTH * line)..];
                    let output = &mut self.ppu.framebuffer[(PIXELS_WIDTH * line)..];
                    for x in 0..PIXELS_WIDTH {
                        let color_index = input[x];
                        let color_15bit = self.ppu.palette.read_16((color_index as u32) * 2);
                        output[x] = pixel16_to_32(color_15bit);
                    }
                }
            }
            m @ _ => panic!("Unsupported video mode {}", m),
        }

        self.ppu.vcount += 1;
        if (self.ppu.vcount as usize) == (PIXELS_HEIGHT + SCANLINES_VBLANK) {
            self.ppu.vcount = 0;
            self.ppu.frame += 1;
        }
        self.ppu.dispstat.vblank = (self.ppu.vcount as usize) >= PIXELS_HEIGHT;
        (PpuEvent::EndScanline, CYCLES_SCANLINE)
    }
}
