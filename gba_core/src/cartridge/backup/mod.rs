use crate::Rom;

mod flash;

pub use flash::{FlashBackup, FlashSize};

#[derive(Copy, Clone, Debug)]

pub enum BackupType {
    /// No backup
    None,

    /// EEPROM 512B or 8KiB
    Eeprom,

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
            (b"EEPROM_V", BackupType::Eeprom),
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
    /// Initialize the file for use, with the given size.
    fn initialize(&mut self, size: usize);

    /// Read bytes from the given offset into the buffer.
    fn read(&mut self, offset: usize, buffer: &mut [u8]);

    /// Write bytes from the given buffer at the offset.
    fn write(&mut self, offset: usize, data: &[u8]);
}

/// Dummy backup file implementation that stores data in memory.
#[derive(Default)]
pub struct MemoryBackupFile {
    pub storage: Vec<u8>,
}

impl BackupFile for MemoryBackupFile {
    fn initialize(&mut self, size: usize) {
        self.storage.resize(size, 0u8);
    }

    fn read(&mut self, offset: usize, buffer: &mut [u8]) {
        buffer.copy_from_slice(&self.storage[offset..(offset + buffer.len())])
    }

    fn write(&mut self, offset: usize, data: &[u8]) {
        self.storage[offset..(offset + data.len())].copy_from_slice(data);
    }
}

/// A concrete cartridge backup.
pub enum Backup {
    None,
    Sram(Box<dyn BackupFile>),
    Eeprom,
    Flash(FlashBackup),
}

impl Backup {
    /// Construct a new backup from a backup file and type.
    pub fn new(backup_type: BackupType, mut file: Box<dyn BackupFile>) -> Backup {
        match backup_type {
            BackupType::None => Backup::None,
            BackupType::Sram => {
                file.initialize(32 * 1024);
                Backup::Sram(file)
            }
            // TODO: implement Eeprom.
            BackupType::Eeprom => Backup::Eeprom,
            BackupType::Flash64K => Backup::Flash(FlashBackup::new(FlashSize::Flash64K, file)),
            BackupType::Flash128K => Backup::Flash(FlashBackup::new(FlashSize::Flash128K, file)),
        }
    }
}
