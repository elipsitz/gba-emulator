mod backup;
mod rom;

pub use backup::{BackupFile, BackupType};
pub use rom::Rom;

use backup::Backup;

use crate::{bus, mem::Memory};

/// A GamePak cartridge.
pub struct Cartridge {
    /// The ROM file.
    pub rom: Rom,
    /// The cartridge backup.
    backup: Backup,
}

impl Cartridge {
    pub fn new(rom: Rom, backup_file: Option<Box<dyn BackupFile>>) -> Cartridge {
        let backup_file =
            backup_file.unwrap_or_else(|| Box::new(backup::MemoryBackupFile::default()));
        let backup_type = BackupType::detect(&rom);
        let backup = Backup::new(backup_type, backup_file);

        Cartridge { rom, backup }
    }
}

impl Memory for Cartridge {
    fn read_8(&mut self, addr: u32) -> u8 {
        match bus::region_from_address(addr) {
            bus::REGION_SRAM | bus::REGION_CART_UNUSED => match &mut self.backup {
                Backup::Sram(file) => {
                    let mut data = 0;
                    file.read((addr & 0x7FFF) as usize, std::slice::from_mut(&mut data));
                    data
                }
                Backup::Flash(flash) => flash.read_8(addr & 0xFFFF),
                _ => 0,
            },
            _ => {
                let addr = (addr & 0x01FF_FFFF) as usize;
                if addr < self.rom.data.len() {
                    self.rom.data[addr]
                } else {
                    // TODO handle invalid cartridge read.
                    0
                }
            }
        }
    }

    fn write_8(&mut self, addr: u32, value: u8) {
        match bus::region_from_address(addr) {
            bus::REGION_SRAM | bus::REGION_CART_UNUSED => match &mut self.backup {
                Backup::Sram(file) => {
                    file.write((addr & 0x7FFF) as usize, &[value]);
                }
                Backup::Flash(flash) => flash.write_8(addr & 0xFFFF, value),
                _ => {}
            },
            _ => {}
        }
    }
}
