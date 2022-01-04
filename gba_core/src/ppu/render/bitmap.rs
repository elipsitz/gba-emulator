use super::super::{constants::*, Color15};
use super::{BackgroundBuffer, PALETTE_TABLE_BG};
use crate::mem::Memory;
use crate::Gba;

impl Gba {
    /// Render bitmap mode 3: 240x160, 16 bpp
    pub(super) fn ppu_render_bitmap_3(&mut self, buffer: &mut BackgroundBuffer) {
        let (w, h) = (240, 160);
        for screen_x in 0..PIXELS_WIDTH {
            if let Some((tx, ty)) = self.bg_affine_transform(2, screen_x as i32, w, h) {
                let address = 2 * ((w as u32) * ty + tx);
                let color = Color15(self.ppu.vram.read_16(address));
                buffer[screen_x] = color;
            }
        }
    }

    /// Render bitmap mode 4: Bitmap: 240x160, 8 bpp (palette) (allows page flipping)
    pub(super) fn ppu_render_bitmap_4(&mut self, buffer: &mut BackgroundBuffer) {
        let (w, h) = (240, 160);
        let page_address = 0xA000 * (self.ppu.dispcnt.display_frame as usize);

        for screen_x in 0..PIXELS_WIDTH {
            if let Some((tx, ty)) = self.bg_affine_transform(2, screen_x as i32, w, h) {
                let address = page_address + (((w as u32) * ty + tx) as usize);
                let index = self.ppu.vram[address];
                let color = self.palette_get_color(index, 0, PALETTE_TABLE_BG);
                buffer[screen_x] = color;
            }
        }
    }

    /// Render bitmap mode 5: 160x128 pixels, 16 bpp, allows page flipping
    pub(super) fn ppu_render_bitmap_5(&mut self, buffer: &mut BackgroundBuffer) {
        let (w, h) = (160, 128);
        let page_address = 0xA000 * (self.ppu.dispcnt.display_frame as u32);

        for screen_x in 0..PIXELS_WIDTH {
            if let Some((tx, ty)) = self.bg_affine_transform(2, screen_x as i32, w, h) {
                let offset = 2 * ((w as u32) * ty + tx);
                let color = Color15(self.ppu.vram.read_16(page_address + offset));
                buffer[screen_x] = color;
            }
        }
    }
}
