mod backup;
mod rom;

pub use backup::{BackupFile, BackupType};
pub use rom::Rom;

use crate::mem::Memory;

/// A GamePak cartridge.
pub struct Cartridge {
    /// The ROM file.
    pub rom: Rom,
}

impl Cartridge {
    pub fn new(rom: Rom) -> Cartridge {
        Cartridge { rom }
    }
}

impl Memory for Cartridge {
    fn read_8(&mut self, addr: u32) -> u8 {
        let addr = (addr & 0x01FF_FFFF) as usize;
        if addr < self.rom.data.len() {
            self.rom.data[addr]
        } else {
            0
        }
    }

    fn write_8(&mut self, _addr: u32, _value: u8) {}
}
