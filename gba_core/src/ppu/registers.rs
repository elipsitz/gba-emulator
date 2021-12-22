use bit::BitIndex;

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
