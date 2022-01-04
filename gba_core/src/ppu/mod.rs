use crate::{
    scheduler::{Event, PpuEvent},
    Gba, InterruptKind, HEIGHT, WIDTH,
};
use color::Color15;
use registers::*;

mod color;
mod registers;
mod render;

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

    /// Registers BGxCNT - Background Control
    pub bgcnt: [BackgroundControl; 4],

    /// Registers BGxHOFS - Background X-Offsets
    pub bg_hofs: [u16; 4],

    /// Registers BGxVOFS - Background Y-Offsets
    pub bg_vofs: [u16; 4],

    /// Background Affine Registers.
    pub bg_affine: [BackgroundAffine; 2],

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
            framebuffer: vec![0; WIDTH * HEIGHT].into_boxed_slice(),
            dispcnt: DisplayControl::default(),
            dispstat: DisplayStatus::default(),
            bgcnt: <[BackgroundControl; 4]>::default(),
            bg_hofs: [0; 4],
            bg_vofs: [0; 4],
            bg_affine: <[BackgroundAffine; 2]>::default(),
            vcount: 0,
            frame: 0,

            // 96KiB, but we'll make it 128KiB for accesses
            vram: vec![0; 128 * 1024].into_boxed_slice(),
            palette: vec![0; 1024].into_boxed_slice(),
            oam: vec![0; 1024].into_boxed_slice(),
        }
    }
}

impl Gba {
    pub fn ppu_init(&mut self) {
        self.scheduler
            .push_event(Event::Ppu(PpuEvent::EndHDraw), CYCLES_HDRAW);
    }

    fn update_vcount(&mut self, new_vcount: u16) {
        self.ppu.vcount = new_vcount;
        self.ppu.dispstat.vcounter = self.ppu.dispstat.vcount_setting == new_vcount;
        if self.ppu.dispstat.vcounter_irq && self.ppu.dispstat.vcounter {
            self.interrupt_raise(InterruptKind::VCount);
        }
    }

    pub fn ppu_on_event(&mut self, event: PpuEvent, lateness: usize) {
        let (next_event, deadline) = match event {
            PpuEvent::EndHDraw => self.ppu_on_end_hdraw(),
            PpuEvent::EndHBlank => self.ppu_on_end_hblank(),
            PpuEvent::EndVBlankHDraw => self.ppu_on_end_vblank_hdraw(),
            PpuEvent::EndVBlankHBlank => self.ppu_on_end_vblank_hblank(),
        };
        let deadline = deadline - lateness;
        self.scheduler.push_event(Event::Ppu(next_event), deadline);
    }

    fn ppu_on_end_hdraw(&mut self) -> (PpuEvent, usize) {
        self.ppu.dispstat.hblank = true;
        if self.ppu.dispstat.hblank_irq {
            self.interrupt_raise(InterruptKind::HBlank);
        }

        (PpuEvent::EndHBlank, CYCLES_HBLANK)
    }

    fn ppu_on_end_hblank(&mut self) -> (PpuEvent, usize) {
        // Increment the scanline.
        self.ppu.dispstat.hblank = false;
        self.update_vcount(self.ppu.vcount + 1);

        if (self.ppu.vcount as usize) == PIXELS_HEIGHT {
            // Just entered vblank.
            self.ppu.dispstat.vblank = true;
            if self.ppu.dispstat.vblank_irq {
                self.interrupt_raise(InterruptKind::VBlank);
            }

            // Copy the affine displacement registers to the internal ones.
            for i in 0..2 {
                self.ppu.bg_affine[i].internal_dx = self.ppu.bg_affine[i].dx;
                self.ppu.bg_affine[i].internal_dy = self.ppu.bg_affine[i].dy;
            }

            (PpuEvent::EndVBlankHDraw, CYCLES_HDRAW)
        } else {
            // Update the affine displacement registers.
            for i in 0..2 {
                self.ppu.bg_affine[i].internal_dx += self.ppu.bg_affine[i].pb as i32;
                self.ppu.bg_affine[i].internal_dy += self.ppu.bg_affine[i].pd as i32;
            }

            // Draw the next scanline (which is visible).
            // XXX: I think we need to fire (and process) vcount interrupt before rendering line.
            // See the red line when vcount has priority in tonc interrupt demo.
            self.ppu_render_scanline();

            (PpuEvent::EndHDraw, CYCLES_HDRAW)
        }
    }

    fn ppu_on_end_vblank_hdraw(&mut self) -> (PpuEvent, usize) {
        self.ppu.dispstat.hblank = true;
        if self.ppu.dispstat.hblank_irq {
            self.interrupt_raise(InterruptKind::HBlank);
        }

        (PpuEvent::EndVBlankHBlank, CYCLES_HBLANK)
    }

    fn ppu_on_end_vblank_hblank(&mut self) -> (PpuEvent, usize) {
        // Increment the scanline.
        self.ppu.dispstat.hblank = false;
        let new_vcount = self.ppu.vcount + 1;

        if (new_vcount as usize) == (PIXELS_HEIGHT + SCANLINES_VBLANK) {
            // Finished vblank.
            self.ppu.dispstat.vblank = false;
            self.update_vcount(0);
            self.ppu.frame += 1;

            // Draw the first scanline.
            self.ppu_render_scanline();

            (PpuEvent::EndHDraw, CYCLES_HDRAW)
        } else {
            // Another vblank scanline.
            self.update_vcount(new_vcount);
            (PpuEvent::EndVBlankHDraw, CYCLES_HDRAW)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorMode {
    /// 4 bits per pixel (16 colors).
    Bpp4 = 0,
    /// 8 bits per pixel (256 colors).
    Bpp8 = 1,
}
