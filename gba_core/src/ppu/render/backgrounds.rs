use super::{BackgroundBuffer, PALETTE_TABLE_BG};
use crate::ppu::{ColorMode, PIXELS_WIDTH};
use crate::{mem::Memory, Gba};
use bit::BitIndex;

#[derive(Copy, Clone)]
struct ScreenEntryRegular(u16);

impl ScreenEntryRegular {
    /// Tile-index of the screen entry.
    fn tile_index(self) -> u16 {
        self.0.bit_range(0..10)
    }

    /// Horizontal flip flag.
    fn hflip(self) -> bool {
        self.0.bit(0xA)
    }

    /// Vertical flip flag.
    fn vflip(self) -> bool {
        self.0.bit(0xB)
    }

    /// Palette bank (in 16 color mode).
    fn palette_bank(self) -> u32 {
        self.0.bit_range(12..16) as u32
    }
}

impl Gba {
    /// Render an affine background in the current scanline.
    pub(super) fn ppu_render_affine_background(
        &mut self,
        index: usize,
        buffer: &mut BackgroundBuffer,
    ) {
        let control = self.ppu.bgcnt[index];
        let (w, h) = control.size.pixels(true);

        for screen_x in 0..PIXELS_WIDTH {
            let transformed = self.bg_affine_transform(index, screen_x as i32, w as i32, h as i32);
            let (tx, ty) = match transformed {
                Some(x) => x,
                None => continue,
            };

            let tile_x = (tx as u32) / 8;
            let tile_y = (ty as u32) / 8;
            let subtile_x = (tx as u32) % 8;
            let subtile_y = (ty as u32) % 8;

            let entry_offset = tile_x + (tile_y * (h as u32) / 8);
            let entry_address_base = 0x800 * (control.screen_base_block as u32);
            let entry_address = entry_address_base + entry_offset;
            let entry = self.ppu.vram[entry_address as usize];

            let tile_address_base = 0x4000 * (control.character_base_block as u32);
            let tile_address = tile_address_base + (0x40 * (entry as u32));
            let index = self.tile_8bpp_get_index(tile_address, subtile_x, subtile_y);
            let color = self.palette_get_color(index, 0, PALETTE_TABLE_BG);
            buffer[screen_x as usize] = color;
        }
    }

    /// Render a regular (non-affine) background in the current scanline.
    pub(super) fn ppu_render_regular_background(
        &mut self,
        index: usize,
        buffer: &mut BackgroundBuffer,
    ) {
        let off_x = self.ppu.bg_hofs[index];
        let off_y = self.ppu.bg_vofs[index];
        let control = self.ppu.bgcnt[index];
        let (w, h) = control.size.pixels(false);

        // Y coordinate of the line of the background we're rendering.
        let bg_y = ((off_y + self.ppu.vcount) as u32) % (h as u32);
        let tile_y = bg_y / 8;
        let subtile_y = bg_y % 8;

        // Address of the base screenblock and tileblock in VRAM.
        let entry_address_base = 0x800 * (control.screen_base_block as u32);
        let tile_address_base = 0x4000 * (control.character_base_block as u32);

        for screen_x in 0..PIXELS_WIDTH {
            // XXX: consider doing optimization to keep the same tile data for 8 pixels.
            let bg_x = ((off_x as u32) + (screen_x as u32)) % (w as u32);
            let tile_x = bg_x / 8;
            let mut subtile_x = bg_x % 8;

            // Compute the screen entry index in the screenblock data.
            let entry_index = {
                // Formula from TONC.
                let mut index = tile_x + (tile_y * 32);
                if tile_x >= 32 {
                    index += 0x03E0;
                }
                if tile_y >= 32 && w == 512 && h == 512 {
                    index += 0x0400;
                }
                index
            };
            let entry_address = entry_address_base + (entry_index * 2);
            let entry = ScreenEntryRegular(self.ppu.vram.read_16(entry_address));

            // Handle flipping.
            if entry.hflip() {
                subtile_x = 7 - subtile_x;
            }
            let subtile_y = if entry.vflip() {
                7 - subtile_y
            } else {
                subtile_y
            };

            // Load the tile data.
            // TODO don't allow accessing data in tileblocks 4-5 (object tileblocks).
            let tile_index = entry.tile_index() as u32;
            let (index, palette_bank) = match control.color_mode {
                ColorMode::Bpp4 => {
                    let address = tile_address_base + (0x20 * tile_index);
                    let index = self.tile_4bpp_get_index(address, subtile_x, subtile_y);
                    (index, entry.palette_bank())
                }
                ColorMode::Bpp8 => {
                    let address = tile_address_base + (0x40 * tile_index);
                    let index = self.tile_8bpp_get_index(address, subtile_x, subtile_y);
                    (index, 0)
                }
            };
            let color = self.palette_get_color(index, palette_bank, PALETTE_TABLE_BG);
            buffer[screen_x as usize] = color;
        }
    }
}
