use std::hint::unreachable_unchecked;

use crate::{mem::Memory, Gba};
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

#[derive(Clone, Copy, Debug, PartialEq)]
enum ColorMode {
    /// 4 bits per pixel (16 colors).
    Bpp4,
    /// 8 bits per pixel (256 colors).
    Bpp8,
}

struct ObjectAttributes {
    raw: [u16; 3],
}

impl ObjectAttributes {
    fn y(&self) -> u32 {
        self.raw[0].bit_range(0..8) as u32
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

    fn x(&self) -> u32 {
        self.raw[1].bit_range(0..9) as u32
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

    fn size(&self) -> (u32, u32) {
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

    fn palette_bank(&self) -> usize {
        self.raw[2].bit_range(12..16) as usize
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

    /// Render the objects in the current scanline.
    pub(super) fn ppu_render_objects(&mut self) {
        for i in 0..128 {
            let attrs = self.get_attributes(i);
            if attrs.object_mode() == ObjectMode::Regular {
                println!("sprite {}", i);
            }
        }
        println!("====");
    }
}
