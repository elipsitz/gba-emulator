use super::constants::*;
use super::Color15;
use crate::{mem::Memory, Gba};

mod objects;

impl Gba {
    /// Render the current scanline.
    pub(super) fn ppu_render_scanline(&mut self) {
        // Clear background.
        let output = &mut self.ppu.framebuffer[(PIXELS_WIDTH * (self.ppu.vcount as usize))..];
        for x in 0..PIXELS_WIDTH {
            output[x] = 0xFF000000;
        }

        self.ppu_render_objects();

        match self.ppu.dispcnt.mode {
            0 => {}
            3 => {
                // Mode 3: Bitmap: 240x160, 16 bpp
                let line = self.ppu.vcount as usize;
                if line < PIXELS_HEIGHT {
                    let input = &mut self.ppu.vram[(PIXELS_WIDTH * line * 2)..];
                    let output = &mut self.ppu.framebuffer[(PIXELS_WIDTH * line)..];
                    for x in 0..PIXELS_WIDTH {
                        let color = Color15(input.read_16((x * 2) as u32));
                        output[x] = color.as_argb();
                    }
                }
            }
            4 => {
                // Mode 4: Bitmap: 240x160, 8 bpp (palette) (allows page flipping)
                let line = self.ppu.vcount as usize;
                if line < PIXELS_HEIGHT {
                    let input = &self.ppu.vram[(PIXELS_WIDTH * line)..];
                    let output = &mut self.ppu.framebuffer[(PIXELS_WIDTH * line)..];
                    for x in 0..PIXELS_WIDTH {
                        let color_index = input[x];
                        let color = Color15(self.ppu.palette.read_16((color_index as u32) * 2));
                        output[x] = color.as_argb();
                    }
                }
            }
            m @ _ => panic!("Unsupported video mode {}", m),
        }
    }
}
