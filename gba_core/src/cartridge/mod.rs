mod backup;
mod rom;

use std::ops::DerefMut;

pub use backup::{BackupFile, BackupType, MemoryBackupFile};
pub use rom::Rom;

use backup::Backup;

use crate::{bus, Gba};

/// State for a GamePak cartridge.
pub struct Cartridge {
    pub backup: Backup,
}

impl Cartridge {
    pub fn new(rom: &Rom) -> Cartridge {
        let backup_type = BackupType::detect(&rom);
        let backup = Backup::new(backup_type);
        Cartridge { backup }
    }
}

impl Gba {
    pub(crate) fn cart_read_8(&mut self, addr: u32) -> u8 {
        match bus::region_from_address(addr) {
            bus::REGION_SRAM | bus::REGION_CART_UNUSED => match &mut self.cartridge.backup {
                Backup::Sram => {
                    let mut data = 0;
                    let file = &mut self.cart_backup_file;
                    file.read((addr & 0x7FFF) as usize, std::slice::from_mut(&mut data));
                    data
                }
                Backup::Flash(flash) => {
                    flash.read_8(addr & 0xFFFF, self.cart_backup_file.deref_mut())
                }
                _ => 0,
            },
            _ => {
                let addr = (addr & 0x01FF_FFFF) as usize;
                if addr < self.cart_rom.data.len() {
                    self.cart_rom.data[addr]
                } else {
                    // Out of bounds cartridge read.
                    // The same signal lines are used for data and the address, causing
                    // the address (sort of) to be read.
                    let data16 = (addr / 2) & 0xFFFF;
                    (data16 >> ((addr & 1) * 8)) as u8
                }
            }
        }
    }

    pub(crate) fn cart_write_8(&mut self, addr: u32, value: u8) {
        match bus::region_from_address(addr) {
            bus::REGION_SRAM | bus::REGION_CART_UNUSED => match &mut self.cartridge.backup {
                Backup::Sram => {
                    let file = &mut self.cart_backup_file;
                    file.write((addr & 0x7FFF) as usize, &[value]);
                }
                Backup::Flash(flash) => {
                    flash.write_8(addr & 0xFFFF, value, self.cart_backup_file.deref_mut());
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub(crate) fn cart_read_16(&mut self, addr: u32) -> u16 {
        (self.cart_read_8(addr) as u16) | ((self.cart_read_8(addr + 1) as u16) << 8)
    }

    pub(crate) fn cart_read_32(&mut self, addr: u32) -> u32 {
        (self.cart_read_16(addr) as u32) | ((self.cart_read_16(addr + 2) as u32) << 16)
    }

    pub(crate) fn cart_write_16(&mut self, addr: u32, value: u16) {
        self.cart_write_8(addr, (value & 0xFF) as u8);
        self.cart_write_8(addr + 1, ((value >> 8) & 0xFF) as u8);
    }

    pub(crate) fn cart_write_32(&mut self, addr: u32, value: u32) {
        self.cart_write_16(addr, (value & 0xFFFF) as u16);
        self.cart_write_16(addr + 2, ((value >> 16) & 0xFFFF) as u16);
    }
}
