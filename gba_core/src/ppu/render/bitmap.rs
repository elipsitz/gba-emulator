use super::super::{constants::*, Color15};
use super::{BackgroundBuffer, PALETTE_TABLE_BG};
use crate::mem::Memory;
use crate::Gba;

impl Gba {
    /// Render bitmap mode 3: 240x160, 16 bpp
    pub(super) fn ppu_render_bitmap_3(&mut self, buffer: &mut BackgroundBuffer) {
        // TODO handle translation using affine offset registers
        let screen_y = self.ppu.vcount as usize;
        let input = &mut self.ppu.vram[(PIXELS_WIDTH * screen_y * 2)..];
        for screen_x in 0..PIXELS_WIDTH {
            let color = Color15(input.read_16((screen_x * 2) as u32));
            buffer[screen_x] = color;
        }
    }

    /// Render bitmap mode 4: Bitmap: 240x160, 8 bpp (palette) (allows page flipping)
    pub(super) fn ppu_render_bitmap_4(&mut self, buffer: &mut BackgroundBuffer) {
        // TODO handle translation using affine offset registers
        let screen_y = self.ppu.vcount as usize;
        let page_address = 0xA000 * (self.ppu.dispcnt.display_frame as usize);
        let base_address = page_address + (PIXELS_WIDTH * screen_y);
        for screen_x in 0..PIXELS_WIDTH {
            let index = self.ppu.vram[base_address + screen_x];
            let color = self.palette_get_color(index, 0, PALETTE_TABLE_BG);
            buffer[screen_x] = color;
        }
    }

    /// Render bitmap mode 5: 160x128 pixels, 16 bpp, allows page flipping
    pub(super) fn ppu_render_bitmap_5(&mut self, buffer: &mut BackgroundBuffer) {
        // TODO handle translation using affine offset registers
        let screen_y = self.ppu.vcount as usize;
        let (w, h) = (160, 128);
        if screen_y >= h {
            return;
        }
        let page_address = 0xA000 * (self.ppu.dispcnt.display_frame as usize);
        let base_address = page_address + ((w * screen_y) * 2);
        let input = &mut self.ppu.vram[base_address..];

        for screen_x in 0..w {
            let color = Color15(input.read_16((screen_x * 2) as u32));
            buffer[screen_x] = color;
        }
    }
}
