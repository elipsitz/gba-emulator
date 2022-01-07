use crate::Rom;

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
    /// Read bytes from the given offset into the buffer.
    fn read(&mut self, offset: usize, buffer: &mut [u8]);

    /// Write bytes from the given buffer at the offset.
    fn write(&mut self, offset: usize, data: &[u8]);
}

/// Dummy backup file implementation that stores data in memory.
pub struct MemoryBackupFile {
    pub storage: Vec<u8>,
}

impl BackupFile for MemoryBackupFile {
    fn read(&mut self, offset: usize, buffer: &mut [u8]) {
        let available = self.storage.len().saturating_sub(offset);
        let read = buffer.len().min(available);
        buffer[..read].copy_from_slice(&self.storage[offset..(offset + read)]);
        buffer[read..].fill(0u8);
    }

    fn write(&mut self, offset: usize, data: &[u8]) {
        let new_len = self.storage.len().max(offset + data.len());
        self.storage.resize(new_len, 0u8);
        self.storage[offset..(offset + data.len())].copy_from_slice(data);
    }
}
