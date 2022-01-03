use super::constants::*;
use super::Color15;
use crate::{mem::Memory, Gba};

mod backgrounds;
mod objects;

const PALETTE_TABLE_BG: u32 = 0x0000;
const PALETTE_TABLE_OBJ: u32 = 0x0200;
const PRIORITY_HIDDEN: u16 = u16::MAX;

/// Entry in the scanline object buffer.
/// Used to keep track of objects and priorities as we're rendering a scanline.
#[derive(Copy, Clone)]
struct ObjectBufferEntry {
    pub color: Color15,
    pub priority: u16,
}

impl ObjectBufferEntry {
    fn set(&mut self, color: Color15, priority: u16) {
        if priority < self.priority {
            self.color = color;
            self.priority = priority;
        }
    }
}

impl Default for ObjectBufferEntry {
    fn default() -> Self {
        Self {
            color: Color15::TRANSPARENT,
            priority: u16::MAX,
        }
    }
}

/// Object scanline buffer.
type ObjectBuffer = [ObjectBufferEntry; PIXELS_WIDTH];

/// Background scanline buffer.
type BackgroundBuffer = [Color15; PIXELS_WIDTH];

impl Gba {
    /// Render the current scanline.
    pub(super) fn ppu_render_scanline(&mut self) {
        // Render objects.
        let mut object_buffer = [ObjectBufferEntry::default(); PIXELS_WIDTH];
        if self.ppu.dispcnt.display_obj {
            self.ppu_render_objects(&mut object_buffer);
        }

        // Render backgrounds.
        let screen_y = self.ppu.vcount as usize;
        let mut background_buffers = [[Color15::TRANSPARENT; PIXELS_WIDTH]; 4];
        let mut background_indices = [0usize; 4];
        let mut background_count = 0;
        match self.ppu.dispcnt.mode {
            0 => {
                // Mode 0: Four regular tilemaps.
                for i in 0..4 {
                    if self.ppu.dispcnt.display_bg[i] {
                        let buffer = &mut background_buffers[i];
                        self.ppu_render_regular_background(i, buffer);
                        background_indices[background_count] = i;
                        background_count += 1;
                    }
                }
            }
            1 => {
                // Mode 1: Two regular tilemaps (0, 1), one affine (2).
                for i in 0..2 {
                    if self.ppu.dispcnt.display_bg[i] {
                        let buffer = &mut background_buffers[i];
                        self.ppu_render_regular_background(i, buffer);
                        background_indices[background_count] = i;
                        background_count += 1;
                    }
                }
                // TODO implement affine tilemaps.
            }
            2 => {
                // Mode 2: Two affine tilemaps (2, 3).
                // TODO implement affine tilemaps.
                background_count = 0;
            }
            3 => {
                // Mode 3: Bitmap: 240x160, 16 bpp
                if self.ppu.dispcnt.display_bg[2] {
                    let input = &mut self.ppu.vram[(PIXELS_WIDTH * screen_y * 2)..];
                    for screen_x in 0..PIXELS_WIDTH {
                        let color = Color15(input.read_16((screen_x * 2) as u32));
                        background_buffers[2][screen_x] = color;
                    }
                    background_indices[0] = 2;
                    background_count = 1;
                }
            }
            4 => {
                // Mode 4: Bitmap: 240x160, 8 bpp (palette) (allows page flipping)
                if self.ppu.dispcnt.display_bg[2] {
                    let page_address = 0xA000 * (self.ppu.dispcnt.display_frame as usize);
                    let base_address = page_address + (PIXELS_WIDTH * screen_y);
                    for screen_x in 0..PIXELS_WIDTH {
                        let index = self.ppu.vram[base_address + screen_x];
                        let color = self.palette_get_color(index, 0, PALETTE_TABLE_BG);
                        background_buffers[2][screen_x] = color;
                    }
                    background_indices[0] = 2;
                    background_count = 1;
                }
            }
            5 => {
                // Mode 5: Bitmap: 160x128 pixels, 16 bpp, allows page flipping
                let (w, h) = (160, 128);
                if self.ppu.dispcnt.display_bg[2] && screen_y < h {
                    let page_address = 0xA000 * (self.ppu.dispcnt.display_frame as usize);
                    let base_address = page_address + ((w * screen_y) * 2);
                    let input = &mut self.ppu.vram[base_address..];

                    for screen_x in 0..w {
                        let color = Color15(input.read_16((screen_x * 2) as u32));
                        background_buffers[2][screen_x] = color;
                    }
                    background_indices[0] = 2;
                    background_count = 1;
                }
            }
            m @ _ => panic!("Unsupported video mode {}", m),
        }

        self.compose_scanline(
            &object_buffer,
            &background_buffers,
            &mut background_indices[..background_count],
        );
    }

    /// Do final composition of a scanline and write it to the screenbuffer.
    fn compose_scanline(
        &mut self,
        object_buffer: &ObjectBuffer,
        background_buffers: &[BackgroundBuffer; 4],
        background_indices: &mut [usize],
    ) {
        let framebuffer_offset = PIXELS_WIDTH * (self.ppu.vcount as usize);
        let backdrop_color = Color15(self.ppu.palette.read_16(0));

        // Sort backgrounds.
        background_indices.sort_by_key(|&x| self.ppu.bgcnt[x].priority);

        // TODO: implement more complex object/background priority interactions.
        for x in 0..PIXELS_WIDTH {
            let mut color = backdrop_color;

            // Find first non-transparent background layer.
            let bg_layer = background_indices
                .iter()
                .filter(|&&i| !background_buffers[i][x].transparent())
                .next();
            let bg_priority = bg_layer.map_or(PRIORITY_HIDDEN, |&i| self.ppu.bgcnt[i].priority);
            if let Some(&layer) = bg_layer {
                color = background_buffers[layer][x];
            }

            // Add object color if it's not transparent.
            if !object_buffer[x].color.transparent() && object_buffer[x].priority <= bg_priority {
                color = object_buffer[x].color;
            }

            self.ppu.framebuffer[framebuffer_offset + x] = color.as_argb();
        }
    }

    /// Get a palette index from a 4bpp tile.
    ///
    /// `address`: the address of the tile in VRAM
    /// `x`: the x coordinate of the pixel in the tile
    /// `y`: the y coordinate of the pixel in the tile
    fn tile_4bpp_get_index(&mut self, address: u32, x: u32, y: u32) -> u8 {
        let pixel = y * 8 + x;
        let address = address + (pixel / 2);
        let data = self.ppu.vram[address as usize];
        if (pixel & 1) == 0 {
            data & 0xF
        } else {
            data >> 4
        }
    }

    /// Get a palette index from an 8bpp tile.
    ///
    /// `address`: the address of the tile in VRAM
    /// `x`: the x coordinate of the pixel in the tile
    /// `y`: the y coordinate of the pixel in the tile
    fn tile_8bpp_get_index(&mut self, address: u32, x: u32, y: u32) -> u8 {
        let pixel = y * 8 + x;
        let address = address + pixel;
        self.ppu.vram[address as usize]
    }

    /// Get a color from a palette.
    ///
    /// `index`: the index of the color in the palette
    /// `bank`: the palette bank
    /// `table`: selects between sprite and bg palettes
    fn palette_get_color(&mut self, index: u8, bank: u32, table: u32) -> Color15 {
        if index == 0 {
            Color15::TRANSPARENT
        } else {
            let address = table + (2 * index as u32) + (32 * bank);
            let raw = self.ppu.palette.read_16(address);
            Color15(raw & 0x7FFF)
        }
    }
}
