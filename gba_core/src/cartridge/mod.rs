mod backup;
mod game_db;
mod gpio;
mod rom;

pub use backup::{BackupFile, BackupType};
pub use rom::Rom;
use serde::{Deserialize, Serialize};

use crate::{bus, Gba};
use backup::{Backup, BackupBuffer};
use gpio::{Gpio, GpioType};

/// State for a GamePak cartridge.
#[derive(Serialize, Deserialize)]
pub struct Cartridge {
    /// State for the current cartridge backup.
    pub backup: Backup,

    /// In memory storage for the backup.
    pub backup_buffer: BackupBuffer,

    /// EEPROM chip address mask.
    eeprom_mask: u32,

    /// State for the cartridge's GPIO (if one exists).
    gpio: Option<Gpio>,
}

impl Cartridge {
    pub fn new(rom: &Rom, backup_type: Option<BackupType>) -> Cartridge {
        let entry = game_db::lookup(&rom.game_code);
        let backup_type = backup_type
            .or(entry.map(|e| e.backup_type))
            .unwrap_or_else(|| BackupType::detect(&rom));
        let gpio_type = entry.and_then(|e| e.gpio_type);

        eprintln!("Cartridge: using backup type {:?}", backup_type);
        eprintln!("Cartridge: using GPIO {:?}", gpio_type);
        let eeprom_mask = if rom.data.len() > 0x0100_0000 {
            // Above 16 MiB.
            0x01FF_FF00
        } else {
            0x0100_0000
        };
        Cartridge {
            backup: Backup::new(backup_type),
            backup_buffer: BackupBuffer::default(),
            eeprom_mask,
            gpio: gpio_type.map(|kind| Gpio::new(kind)),
        }
    }

    /// Returns whether an address would go to the eeprom.
    /// Only valid if the save type is EEPROM.
    fn is_eeprom(&self, addr: u32) -> bool {
        (addr & self.eeprom_mask) == self.eeprom_mask
    }

    /// Returns whether an address would go to GPIO.
    fn is_gpio(&self, addr: u32) -> bool {
        let addr = addr & 0x01FF_FFFF;
        addr >= 0xC4 && addr <= 0xC9
    }
}

impl Gba {
    pub(crate) fn cart_read_8(&mut self, addr: u32) -> u8 {
        let backup_buffer = &mut self.cartridge.backup_buffer;
        match bus::region_from_address(addr) {
            bus::REGION_SRAM | bus::REGION_CART_UNUSED => match &mut self.cartridge.backup {
                Backup::Sram => backup_buffer.read((addr & 0x7FFF) as usize),
                Backup::Flash(flash) => flash.read_8(addr & 0xFFFF, backup_buffer),
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
        let backup_buffer = &mut self.cartridge.backup_buffer;
        match bus::region_from_address(addr) {
            bus::REGION_SRAM | bus::REGION_CART_UNUSED => match &mut self.cartridge.backup {
                Backup::Sram => {
                    backup_buffer.write((addr & 0x7FFF) as usize, value);
                }
                Backup::Flash(flash) => {
                    flash.write_8(addr & 0xFFFF, value, backup_buffer);
                }
                _ => {}
            },
            _ => {}
        }
    }

    pub(crate) fn cart_read_16(&mut self, addr: u32) -> u16 {
        // Check if we're reading from EEPROM.
        if self.cartridge.is_eeprom(addr) {
            if let Backup::Eeprom(eeprom) = &mut self.cartridge.backup {
                return eeprom.read(&mut self.cartridge.backup_buffer);
            }
        }

        // Check if we're reading from GPIO.
        if self.cartridge.is_gpio(addr) {
            if let Some(gpio) = &mut self.cartridge.gpio {
                if let Some(data) = gpio.read(addr & 0x01FF_FFFF) {
                    return data;
                }
            }
        }

        (self.cart_read_8(addr) as u16) | ((self.cart_read_8(addr + 1) as u16) << 8)
    }

    pub(crate) fn cart_read_32(&mut self, addr: u32) -> u32 {
        (self.cart_read_16(addr) as u32) | ((self.cart_read_16(addr + 2) as u32) << 16)
    }

    pub(crate) fn cart_write_16(&mut self, addr: u32, value: u16) {
        // Check if we're writing to EEPROM.
        if self.cartridge.is_eeprom(addr) {
            if let Backup::Eeprom(eeprom) = &mut self.cartridge.backup {
                eeprom.write(value, &self.dma, &mut self.cartridge.backup_buffer);
            }
        }

        // Check if we're writing to GPIO.
        if self.cartridge.is_gpio(addr) {
            if let Some(gpio) = &mut self.cartridge.gpio {
                gpio.write(addr & 0x01FF_FFFF, value);
                return;
            }
        }

        self.cart_write_8(addr, (value & 0xFF) as u8);
        self.cart_write_8(addr + 1, ((value >> 8) & 0xFF) as u8);
    }

    pub(crate) fn cart_write_32(&mut self, addr: u32, value: u32) {
        self.cart_write_16(addr, (value & 0xFFFF) as u16);
        self.cart_write_16(addr + 2, ((value >> 16) & 0xFFFF) as u16);
    }
}
