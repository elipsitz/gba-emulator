use std::hint::unreachable_unchecked;

use super::super::constants::*;
use super::{AffineMatrix, ObjectBuffer, PALETTE_TABLE_OBJ};
use crate::ppu::{ColorMode, PIXELS_WIDTH};
use crate::{mem::Memory, ppu::color::Color15, Gba};
use bit::BitIndex;

#[derive(Copy, Clone, Debug, PartialEq)]
enum ObjectMode {
    Regular = 0b00,
    Affine = 0b01,
    Hide = 0b10,
    AffineDouble = 0b11,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GraphicsMode {
    Normal = 0b00,
    Blend = 0b01,
    Window = 0b10,
    Forbidden = 0b11,
}

struct ObjectAttributes {
    raw: [u16; 3],
}

impl ObjectAttributes {
    fn pos(&self) -> (i32, i32) {
        let mut x = self.raw[1].bit_range(0..9) as i32;
        let mut y = self.raw[0].bit_range(0..8) as i32;
        if x >= (PIXELS_WIDTH as i32) {
            x -= 512;
        }
        if y >= (PIXELS_HEIGHT as i32) {
            y -= 256;
        }
        (x, y)
    }

    fn object_mode(&self) -> ObjectMode {
        match self.raw[0].bit_range(8..10) {
            0b00 => ObjectMode::Regular,
            0b01 => ObjectMode::Affine,
            0b10 => ObjectMode::Hide,
            0b11 => ObjectMode::AffineDouble,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn gfx_mode(&self) -> GraphicsMode {
        match self.raw[0].bit_range(10..12) {
            0b00 => GraphicsMode::Normal,
            0b01 => GraphicsMode::Blend,
            0b10 => GraphicsMode::Window,
            0b11 => GraphicsMode::Forbidden,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn affine(&self) -> bool {
        self.raw[0].bit(8)
    }

    fn mosaic(&self) -> bool {
        self.raw[0].bit(0xC)
    }

    fn color_mode(&self) -> ColorMode {
        if self.raw[0].bit(0xD) {
            ColorMode::Bpp8
        } else {
            ColorMode::Bpp4
        }
    }

    /// OAM_AFF_ENTY this sprite uses, valid only if sprite is affine.
    fn affine_index(&self) -> usize {
        self.raw[1].bit_range(9..14) as usize
    }

    /// Only valid if sprite isn't affine.
    fn h_flip(&self) -> bool {
        self.raw[1].bit(0xC)
    }

    /// Only valid if sprite isn't affine.
    fn v_flip(&self) -> bool {
        self.raw[1].bit(0xd)
    }

    fn size(&self) -> (i32, i32) {
        let shape = self.raw[0].bit_range(14..16);
        let size = self.raw[1].bit_range(14..16);
        match (shape, size) {
            (0b00, 0b00) => (8, 8),
            (0b01, 0b00) => (16, 8),
            (0b10, 0b00) => (8, 16),
            (0b00, 0b01) => (16, 16),
            (0b01, 0b01) => (32, 8),
            (0b10, 0b01) => (8, 32),
            (0b00, 0b10) => (32, 32),
            (0b01, 0b10) => (32, 16),
            (0b10, 0b10) => (16, 32),
            (0b00, 0b11) => (64, 64),
            (0b01, 0b11) => (64, 32),
            (0b10, 0b11) => (32, 64),
            _ => (8, 8), // "weird behavior"
        }
    }

    /// Base tile index. Must be >= 512 in bitmap modes.
    fn tile_index(&self) -> usize {
        self.raw[2].bit_range(0..10) as usize
    }

    fn priority(&self) -> u16 {
        self.raw[2].bit_range(10..12)
    }

    fn palette_bank(&self) -> u8 {
        self.raw[2].bit_range(12..16) as u8
    }
}

impl Gba {
    /// Get object attributes for a specific OAM index.
    fn get_attributes(&mut self, index: usize) -> ObjectAttributes {
        let offset = (index as u32) * 8;
        let raw = [
            self.ppu.oam.read_16(offset + 0),
            self.ppu.oam.read_16(offset + 2),
            self.ppu.oam.read_16(offset + 4),
        ];
        ObjectAttributes { raw }
    }

    /// Get an affine matrix by index.
    fn get_affine_matrix(&mut self, index: usize) -> AffineMatrix {
        let address = (index as u32) * 32;
        AffineMatrix {
            pa: self.ppu.oam.read_16(address + 6) as i16 as i32,
            pb: self.ppu.oam.read_16(address + 14) as i16 as i32,
            pc: self.ppu.oam.read_16(address + 22) as i16 as i32,
            pd: self.ppu.oam.read_16(address + 30) as i16 as i32,
        }
    }

    /// Render a normal (non-affine) object.
    fn render_normal_object(&mut self, attrs: ObjectAttributes, buffer: &mut ObjectBuffer) {
        let screen_y = self.ppu.vcount as i32;
        let ((obj_x, obj_y), (obj_w, obj_h)) = (attrs.pos(), attrs.size());
        if screen_y < obj_y || screen_y >= (obj_y + obj_h) {
            // Sprite isn't in this scanline.
            return;
        }
        let left = obj_x.max(0).min(PIXELS_WIDTH as i32);
        let right = (obj_x + obj_w).max(0).min(PIXELS_WIDTH as i32);

        let palette_bank = match attrs.color_mode() {
            ColorMode::Bpp4 => attrs.palette_bank() as u32,
            ColorMode::Bpp8 => 0u32,
        };
        let priority = attrs.priority();

        // Y relative to sprite top.
        let mut sprite_y = screen_y - obj_y;
        if attrs.v_flip() {
            sprite_y = obj_h - sprite_y - 1
        }

        // Left-most tile index of the sprite at this scanline.
        let tile_start = {
            let tile_y = sprite_y / 8; // Y coordinate (in tiles) we're looking at.
            let tile_stride = if self.ppu.dispcnt.obj_character_vram_mapping {
                // 1-D mapping, stride is width in tiles.
                obj_w / 8
            } else {
                // 2-D mapping, stride is 32 tiles.
                32
            };
            attrs.tile_index() + ((tile_y * tile_stride) as usize)
        };
        let subtile_y = (sprite_y % 8) as u32; // Y within the current tile.

        for screen_x in left..right {
            // X relative to sprite left.
            let mut sprite_x = screen_x - obj_x;
            if attrs.h_flip() {
                sprite_x = obj_w - sprite_x - 1;
            }

            let tile_x = sprite_x / 8; // Tile x within the current sprite.
            let subtile_x = (sprite_x % 8) as u32; // X within the current tile.
            let tile_index = (tile_start + (tile_x as usize)) % 1024; // Index of the current tile.
                                                                      // TODO if using bitmap mode and tile_index < 512, don't draw it.

            let tile_address = (0x10000 + (tile_index * 32)) as u32;
            let index = match attrs.color_mode() {
                ColorMode::Bpp4 => self.tile_4bpp_get_index(tile_address, subtile_x, subtile_y),
                ColorMode::Bpp8 => self.tile_8bpp_get_index(tile_address, subtile_x, subtile_y),
            };
            let color = self.palette_get_color(index, palette_bank, PALETTE_TABLE_OBJ);
            if color != Color15::TRANSPARENT {
                buffer[screen_x as usize].set(color, priority);
            }
        }
    }

    /// Render an affine object.
    fn render_affine_object(&mut self, attrs: ObjectAttributes, buffer: &mut ObjectBuffer) {
        let screen_y = self.ppu.vcount as i32;
        let ((obj_x, obj_y), (obj_w, obj_h)) = (attrs.pos(), attrs.size());
        let (box_w, box_h) = if attrs.object_mode() == ObjectMode::Affine {
            (obj_w, obj_h)
        } else {
            (obj_w * 2, obj_h * 2)
        };
        if screen_y < obj_y || screen_y >= (obj_y + box_h) {
            // Sprite isn't in this scanline.
            return;
        }
        let matrix = self.get_affine_matrix(attrs.affine_index());

        // Tile mapping stuff.
        let tile_stride = if self.ppu.dispcnt.obj_character_vram_mapping {
            // 1-D mapping, stride is width in tiles.
            (obj_w as u32) / 8
        } else {
            // 2-D mapping, stride is 32 tiles.
            32
        };
        let palette_bank = match attrs.color_mode() {
            ColorMode::Bpp4 => attrs.palette_bank() as u32,
            ColorMode::Bpp8 => 0u32,
        };
        let priority = attrs.priority();

        let half_width = box_w / 2;
        let half_height = box_h / 2;

        let left = obj_x.max(0).min(PIXELS_WIDTH as i32);
        let right = (obj_x + box_w).max(0).min(PIXELS_WIDTH as i32);
        let iy = screen_y - obj_y - half_height;

        for screen_x in left..right {
            // Apply the transformation.
            let ix = screen_x - obj_x - half_width;
            let texture_x = ((matrix.pa * ix + matrix.pb * iy) >> 8) + (obj_w / 2);
            let texture_y = ((matrix.pc * ix + matrix.pd * iy) >> 8) + (obj_h / 2);

            if texture_x >= 0 && texture_x < obj_w && texture_y >= 0 && texture_y < obj_h {
                let tile_x = (texture_x / 8) as u32;
                let tile_y = (texture_y / 8) as u32;
                let subtile_x = (texture_x % 8) as u32;
                let subtile_y = (texture_y % 8) as u32;

                let tile_offset = tile_x + (tile_y * tile_stride); // Index within sprite.
                let tile_index = attrs.tile_index() + (tile_offset as usize);
                let tile_address = (0x10000 + (tile_index * 32)) as u32;
                let index = match attrs.color_mode() {
                    ColorMode::Bpp4 => self.tile_4bpp_get_index(tile_address, subtile_x, subtile_y),
                    ColorMode::Bpp8 => self.tile_8bpp_get_index(tile_address, subtile_x, subtile_y),
                };
                let color = self.palette_get_color(index, palette_bank, PALETTE_TABLE_OBJ);
                if color != Color15::TRANSPARENT {
                    buffer[screen_x as usize].set(color, priority);
                }
            }
        }
    }

    /// Render the objects in the current scanline.
    pub(super) fn ppu_render_objects(&mut self, buffer: &mut ObjectBuffer) {
        for i in 0..128 {
            let attrs = self.get_attributes(i);
            match attrs.object_mode() {
                ObjectMode::Regular => self.render_normal_object(attrs, buffer),
                ObjectMode::Hide => {}
                ObjectMode::Affine | ObjectMode::AffineDouble => {
                    self.render_affine_object(attrs, buffer)
                }
            }
        }
    }
}
