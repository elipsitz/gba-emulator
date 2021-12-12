use crate::Rom;

/// Game Boy Advance Emulator
pub struct Gba {
    /// The 16 KiB BIOS ROM.
    bios_rom: Box<[u8]>,

    /// The cartridge ROM.
    cart_rom: Rom,

    /// On-board ("external") work RAM.
    ewram: [u8; 256 * 1024],

    /// On-chip ("internal") work RAM.
    iwram: [u8; 32 * 1024],
}

impl Gba {
    pub fn new(bios_rom: Box<[u8]>, cart_rom: Rom) -> Gba {
        Gba {
            bios_rom,
            cart_rom,
            ewram: [0; 256 * 1024],
            iwram: [0; 32 * 1024],
        }
    }
}
