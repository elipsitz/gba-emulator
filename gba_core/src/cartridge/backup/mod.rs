use crate::Rom;

mod eeprom;
mod flash;

pub use eeprom::{EepromBackup, EepromSize};
pub use flash::{FlashBackup, FlashSize};

#[derive(Copy, Clone, Debug)]

pub enum BackupType {
    /// No backup
    None,

    /// EEPROM - Autodetect Size
    EepromAuto,

    /// EEPROM, 512B
    Eeprom512,

    /// EEPROM, 8KiB
    Eeprom8K,

    /// SRAM or FRAM, 32 KiB
    Sram,

    /// Flash 64KiB
    Flash64K,

    /// Flash 128KiB
    Flash128K,
}

impl BackupType {
    pub fn detect(rom: &Rom) -> BackupType {
        static PATTERNS: &[(&[u8], BackupType)] = &[
            (b"EEPROM_V", BackupType::EepromAuto),
            (b"SRAM_V", BackupType::Sram),
            (b"SRAM_F_V", BackupType::Sram),
            (b"FLASH_V", BackupType::Flash64K),
            (b"FLASH512_V", BackupType::Flash64K),
            (b"FLASH1M_V", BackupType::Flash128K),
        ];
        let data = &rom.data;
        for start in (0..data.len()).step_by(4) {
            let region = &data[start..];
            for &(pattern, type_) in PATTERNS {
                if region.starts_with(pattern) {
                    return type_;
                }
            }
        }
        BackupType::None
    }
}

/// Backing storage for the cartridge backup.
pub trait BackupFile {
    /// Get the size of the file.
    fn size(&self) -> usize;

    /// Read bytes from the given offset into the buffer.
    fn read(&mut self, offset: usize, buffer: &mut [u8]);

    /// Write bytes from the given buffer at the offset.
    fn write(&mut self, offset: usize, data: &[u8]);
}

/// In-memory buffer for the backup file.
#[derive(Default)]
pub struct BackupBuffer {
    pub storage: Vec<u8>,

    /// Whether the buffer has unwritten data.
    pub dirty: bool,
}

impl BackupBuffer {
    /// Read a byte from the backup buffer.
    pub fn read(&mut self, address: usize) -> u8 {
        if address < self.storage.len() {
            self.storage[address]
        } else {
            0xFF
        }
    }

    /// Write a byte to the backup buffer.
    pub fn write(&mut self, address: usize, data: u8) {
        if address >= self.storage.len() {
            self.storage.resize(address + 1, 0xFF);
        }
        self.storage[address] = data;
        self.dirty = true;
    }

    /// Persist any unwritten data to the file.
    pub fn save(&self, file: &mut dyn BackupFile) {
        if self.dirty {
            file.write(0, &self.storage);
        }
    }

    /// Load from the backup file.
    pub fn load(&mut self, file: &mut dyn BackupFile) {
        let size = file.size();
        self.storage.resize(size, 0xFF);
        file.read(0, &mut self.storage);
    }
}

/// A concrete cartridge backup.
pub enum Backup {
    None,
    Sram,
    Eeprom(EepromBackup),
    Flash(FlashBackup),
}

impl Backup {
    /// Construct a new backup state from a backup type.
    pub fn new(backup_type: BackupType) -> Backup {
        match backup_type {
            BackupType::None => Backup::None,
            BackupType::Sram => Backup::Sram,
            BackupType::EepromAuto => Backup::Eeprom(EepromBackup::new(None)),
            BackupType::Eeprom512 => Backup::Eeprom(EepromBackup::new(Some(EepromSize::Eeprom512))),
            BackupType::Eeprom8K => Backup::Eeprom(EepromBackup::new(Some(EepromSize::Eeprom8K))),
            BackupType::Flash64K => Backup::Flash(FlashBackup::new(FlashSize::Flash64K)),
            BackupType::Flash128K => Backup::Flash(FlashBackup::new(FlashSize::Flash128K)),
        }
    }
}
