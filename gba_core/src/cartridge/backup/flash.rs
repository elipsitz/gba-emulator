use super::BackupFile;

#[derive(Copy, Clone)]
pub enum FlashSize {
    Flash64K,
    Flash128K,
}

/// Panasonic 64KiB Flash ID
const FLASH_64K_ID: [u8; 2] = [0x32, 0x1B];

/// Sanyo 128KiB Flash ID
const FLASH_128K_ID: [u8; 2] = [0x62, 0x13];

impl FlashSize {
    fn bytes(self) -> usize {
        match self {
            FlashSize::Flash64K => 64 * 1024,
            FlashSize::Flash128K => 128 * 1024,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum CommandState {
    Ready,
    Setup1,
    Setup2,
}

/// A flash backup.
pub struct FlashBackup {
    size: FlashSize,
    file: Box<dyn BackupFile>,

    /// Current Flash command state.
    command: CommandState,

    /// True if we're in "chip identification mode".
    chip_identification: bool,
}

impl FlashBackup {
    pub fn new(size: FlashSize, mut file: Box<dyn BackupFile>) -> FlashBackup {
        file.initialize(size.bytes());
        FlashBackup {
            size,
            file,
            command: CommandState::Ready,
            chip_identification: false,
        }
    }

    pub fn read_8(&mut self, addr: u32) -> u8 {
        // println!("Flash read addr={:04X}", addr);
        if self.chip_identification && addr < 2 {
            let id = match self.size {
                FlashSize::Flash64K => FLASH_64K_ID,
                FlashSize::Flash128K => FLASH_128K_ID,
            };
            id[addr as usize]
        } else {
            // TODO implement reading.
            0
        }
    }

    pub fn write_8(&mut self, addr: u32, data: u8) {
        use CommandState::*;
        // println!("Flash write addr={:04X} data={:02X}", addr, data);
        match (self.command, addr, data) {
            (Ready, 0x5555, 0xAA) => self.command = Setup1,
            (Setup1, 0x2AAA, 0x55) => self.command = Setup2,
            (Setup2, 0x5555, 0x90) => {
                // Enter chip identification mode.
                self.chip_identification = true;
                self.command = Ready;
            }
            (Setup2, 0x5555, 0xF0) => {
                // Exit chip identification mode.
                self.chip_identification = false;
                self.command = Ready;
            }
            _ => {
                eprintln!(
                    "Invalid FLASH write. command={:?}, addr={:04X}, data={:02X}",
                    self.command, addr, data
                );
            }
        }
    }
}
