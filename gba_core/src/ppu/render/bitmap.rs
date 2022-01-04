use super::super::{constants::*, Color15};
use super::{BackgroundBuffer, PALETTE_TABLE_BG};
use crate::mem::Memory;
use crate::Gba;

impl Gba {
    /// Get the texture coord for the given screen X coordinate given bitmap background 2.
    fn get_texture_coord(&mut self, screen_x: i32, w: i32, h: i32) -> Option<(u32, u32)> {
        let control = self.ppu.bgcnt[2];
        let affine = self.ppu.bg_affine[0];
        let (dx, dy) = (affine.internal_dx, affine.internal_dy);

        let tx = (dx + (screen_x as i32) * (affine.pa as i32)) >> 8;
        let ty = (dy + (screen_x as i32) * (affine.pc as i32)) >> 8;
        if tx < 0 || tx >= w || ty < 0 || ty >= h {
            if control.affine_wrap {
                Some((tx.rem_euclid(w) as u32, ty.rem_euclid(h) as u32))
            } else {
                None
            }
        } else {
            Some((tx as u32, ty as u32))
        }
    }

    /// Render bitmap mode 3: 240x160, 16 bpp
    pub(super) fn ppu_render_bitmap_3(&mut self, buffer: &mut BackgroundBuffer) {
        let (w, h) = (240, 160);
        for screen_x in 0..PIXELS_WIDTH {
            if let Some((tx, ty)) = self.get_texture_coord(screen_x as i32, w, h) {
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
            if let Some((tx, ty)) = self.get_texture_coord(screen_x as i32, w, h) {
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
            if let Some((tx, ty)) = self.get_texture_coord(screen_x as i32, w, h) {
                let offset = 2 * ((w as u32) * ty + tx);
                let color = Color15(self.ppu.vram.read_16(page_address + offset));
                buffer[screen_x] = color;
            }
        }
    }
}
