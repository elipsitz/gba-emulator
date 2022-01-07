use bit::BitIndex;

use super::ColorMode;

/// DISPCNT - LCD Control
#[derive(Default)]
pub struct DisplayControl {
    /// Video mode.
    pub mode: u16,

    /// Display frame select (BG modes 4, 5)
    pub display_frame: u16,

    /// H-Blank Interval Free (allow access to OAM during h-blank)
    pub h_blank_interval_free: bool,

    /// OBJ Character VRAM Mapping (false: 2-D, true: 1-D)
    pub obj_character_vram_mapping: bool,

    /// Forced blank (allow FAST access to VRAM, Palette, OAM)
    pub forced_blank: bool,

    /// Screen display BG layer.
    pub display_bg: [bool; 4],

    /// Screen display OBJ layer.
    pub display_obj: bool,

    /// Window 0-1 display
    pub window_display: [bool; 2],

    /// OBJ window display
    pub obj_window_display: bool,
}

impl DisplayControl {
    pub fn write(&mut self, val: u16) {
        self.mode = val.bit_range(0..3);
        self.display_frame = val.bit(4) as u16;
        self.h_blank_interval_free = val.bit(5);
        self.obj_character_vram_mapping = val.bit(6);
        self.forced_blank = val.bit(7);
        self.display_bg[0] = val.bit(8);
        self.display_bg[1] = val.bit(9);
        self.display_bg[2] = val.bit(10);
        self.display_bg[3] = val.bit(11);
        self.display_obj = val.bit(12);
        self.window_display[0] = val.bit(13);
        self.window_display[1] = val.bit(14);
        self.obj_window_display = val.bit(15);
    }

    pub fn read(&self) -> u16 {
        // Bit 3 is 0 (to signify GBA, not CGB)
        (self.mode << 0)
            | ((self.display_frame as u16) << 4)
            | ((self.h_blank_interval_free as u16) << 5)
            | ((self.obj_character_vram_mapping as u16) << 6)
            | ((self.forced_blank as u16) << 7)
            | ((self.display_bg[0] as u16) << 8)
            | ((self.display_bg[1] as u16) << 9)
            | ((self.display_bg[2] as u16) << 10)
            | ((self.display_bg[3] as u16) << 11)
            | ((self.display_obj as u16) << 12)
            | ((self.window_display[0] as u16) << 13)
            | ((self.window_display[1] as u16) << 14)
            | ((self.obj_window_display as u16) << 15)
    }
}

/// DISPSTAT - General LCD Status
#[derive(Default)]
pub struct DisplayStatus {
    /// True during vblank (160..=226 only).
    pub vblank: bool,
    /// True during hblank (toggled in all lines).
    pub hblank: bool,
    /// True when counter matches selected.
    pub vcounter: bool,
    /// V-Blank IRQ Enable
    pub vblank_irq: bool,
    /// H-Blank IRQ Enable
    pub hblank_irq: bool,
    /// V-Counter IRQ Enable
    pub vcounter_irq: bool,
    /// V-Count Setting (LYC) -- 0..=227
    pub vcount_setting: u16,
}

impl DisplayStatus {
    pub fn write(&mut self, val: u16) {
        self.vblank_irq = val.bit(3);
        self.hblank_irq = val.bit(4);
        self.vcounter_irq = val.bit(5);
        self.vcount_setting = val.bit_range(8..16);
    }

    pub fn read(&self) -> u16 {
        ((self.vblank as u16) << 0)
            | ((self.hblank as u16) << 1)
            | ((self.vcounter as u16) << 2)
            | ((self.vblank_irq as u16) << 3)
            | ((self.hblank_irq as u16) << 4)
            | ((self.vcounter_irq as u16) << 5)
            | (self.vcount_setting << 8)
    }
}

#[derive(Copy, Clone)]
pub struct BackgroundSize(u16);

impl BackgroundSize {
    /// Returns the size of the background in pixels.
    pub fn pixels(self, affine: bool) -> (usize, usize) {
        let (w, h) = self.tiles(affine);
        (w * 8, h * 8)
    }

    /// Returns the size of the background in tiles.
    pub fn tiles(self, affine: bool) -> (usize, usize) {
        match (affine, self.0) {
            (false, 0b00) => (32, 32),
            (false, 0b01) => (64, 32),
            (false, 0b10) => (32, 64),
            (false, 0b11) => (64, 64),
            (true, 0b00) => (16, 16),
            (true, 0b01) => (32, 32),
            (true, 0b10) => (64, 64),
            (true, 0b11) => (128, 128),
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

/// BGxCNT - Background Control
#[derive(Copy, Clone)]
pub struct BackgroundControl {
    /// BG Priority
    pub priority: u16,
    /// Character base block.
    /// Charblock serving as base for character/tile indexing.
    pub character_base_block: u16,
    /// Mosaic.
    pub mosaic: bool,
    /// Color mode.
    pub color_mode: ColorMode,
    /// Screen base block.
    /// Screenblock serving as base for screenentry/map indexing.
    pub screen_base_block: u16,
    /// Affine wrapping. If true, affine backgrounds wrap at edges.
    pub affine_wrap: bool,
    /// Background size.
    pub size: BackgroundSize,
}

impl Default for BackgroundControl {
    fn default() -> Self {
        BackgroundControl {
            priority: 0,
            character_base_block: 0,
            mosaic: false,
            color_mode: ColorMode::Bpp4,
            screen_base_block: 0,
            affine_wrap: false,
            size: BackgroundSize(0),
        }
    }
}

impl BackgroundControl {
    pub fn write(&mut self, val: u16) {
        self.priority = val.bit_range(0..2);
        self.character_base_block = val.bit_range(2..4);
        self.mosaic = val.bit(6);
        self.color_mode = if val.bit(7) {
            ColorMode::Bpp8
        } else {
            ColorMode::Bpp4
        };
        self.screen_base_block = val.bit_range(8..13);
        self.affine_wrap = val.bit(13);
        self.size = BackgroundSize(val.bit_range(14..16));
    }

    pub fn read(&self) -> u16 {
        (self.priority << 0)
            | (self.character_base_block << 2)
            | ((self.mosaic as u16) << 6)
            | ((self.color_mode as u16) << 7)
            | (self.screen_base_block << 8)
            | ((self.affine_wrap as u16) << 13)
            | (self.size.0 << 14)
    }
}

/// Affine background registers.
#[derive(Default, Copy, Clone, Debug)]
pub struct BackgroundAffine {
    pub pa: i16,
    pub pb: i16,
    pub pc: i16,
    pub pd: i16,
    pub dx: i32,
    pub dy: i32,
    pub internal_dx: i32,
    pub internal_dy: i32,
}

/// MOSAIC - Mosaic size.
#[derive(Copy, Clone, Debug)]
pub struct Mosaic {
    /// BG mosaic actual h-size.
    pub bg_x: u8,
    /// BG mosaic actual v-size.
    pub bg_y: u8,
    /// OBJ mosaic actual h-size.
    pub obj_x: u8,
    /// OBJ mosaic actual v-size.
    pub obj_y: u8,
}

impl Default for Mosaic {
    fn default() -> Self {
        Mosaic {
            bg_x: 1,
            bg_y: 1,
            obj_x: 1,
            obj_y: 1,
        }
    }
}

impl Mosaic {
    pub fn write(&mut self, val: u16) {
        self.bg_x = (val.bit_range(0..4) as u8) + 1;
        self.bg_y = (val.bit_range(4..8) as u8) + 1;
        self.obj_x = (val.bit_range(8..12) as u8) + 1;
        self.obj_y = (val.bit_range(12..16) as u8) + 1;
    }
}
