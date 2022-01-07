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
