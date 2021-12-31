use super::constants::*;
use super::Color15;
use crate::{mem::Memory, Gba};

mod objects;

const PALETTE_TABLE_BG: u32 = 0x0000;
const PALETTE_TABLE_OBJ: u32 = 0x0200;

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

impl Gba {
    /// Render the current scanline.
    pub(super) fn ppu_render_scanline(&mut self) {
        // Render objects.
        let mut object_buffer = [ObjectBufferEntry::default(); PIXELS_WIDTH];
        self.ppu_render_objects(&mut object_buffer);

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

        self.compose_scanline(&object_buffer);
    }

    /// Do final composition of a scanline and write it to the screenbuffer.
    fn compose_scanline(&mut self, object_buffer: &ObjectBuffer) {
        let framebuffer_offset = PIXELS_WIDTH * (self.ppu.vcount as usize);
        let backdrop_color = Color15(self.ppu.palette.read_16(0));

        for x in 0..PIXELS_WIDTH {
            let mut color = backdrop_color;

            // TODO handle backgrounds properly
            if object_buffer[x].color != Color15::TRANSPARENT {
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
