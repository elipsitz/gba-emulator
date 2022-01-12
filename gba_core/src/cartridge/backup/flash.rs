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

    fn id(self) -> [u8; 2] {
        match self {
            FlashSize::Flash64K => FLASH_64K_ID,
            FlashSize::Flash128K => FLASH_128K_ID,
        }
    }

    fn banks(self) -> u8 {
        match self {
            FlashSize::Flash64K => 1,
            FlashSize::Flash128K => 2,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum CommandState {
    Ready,
    Setup1,
    Setup2,
    BankSwap,
    WriteByte,
}

/// A flash backup.
pub struct FlashBackup {
    size: FlashSize,

    /// Current Flash command state.
    command: CommandState,

    /// True if we're in "chip identification mode".
    chip_identification: bool,

    /// Bank for 128KB chips: (0 or 1).
    bank: u8,

    /// Whether the next command will be an erase command.
    erase_mode: bool,
}

impl FlashBackup {
    pub fn new(size: FlashSize) -> FlashBackup {
        FlashBackup {
            size,
            command: CommandState::Ready,
            chip_identification: false,
            bank: 0,
            erase_mode: false,
        }
    }

    pub fn initialize_file(&self, file: &mut dyn BackupFile) {
        file.initialize(self.size.bytes());
    }

    pub fn read_8(&mut self, addr: u32, file: &mut dyn BackupFile) -> u8 {
        if self.chip_identification && addr < 2 {
            self.size.id()[addr as usize]
        } else {
            let offset = self.address(addr & 0xFFFF);
            let mut data = 0;
            file.read(offset, std::slice::from_mut(&mut data));
            data
        }
    }

    pub fn write_8(&mut self, addr: u32, data: u8, file: &mut dyn BackupFile) {
        use CommandState::*;
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
            (Setup2, 0x5555, 0xB0) => self.command = BankSwap,
            (BankSwap, 0x0000, bank) => {
                self.bank = bank % self.size.banks();
                self.command = Ready;
            }
            (Setup2, 0x5555, 0xA0) => self.command = WriteByte,
            (WriteByte, address, data) => {
                let offset = self.address(address & 0xFFFF);
                file.write(offset, &[data]);
                self.command = Ready;
            }
            (Setup2, 0x5555, 0x80) => {
                // Prepare to erase.
                self.erase_mode = true;
                self.command = Ready;
            }
            (Setup2, 0x5555, 0x10) => {
                // Erase entire chip.
                if self.erase_mode {
                    for i in 0..self.size.bytes() {
                        file.write(i, &[0xFF]);
                    }
                }
                self.command = Ready;
                self.erase_mode = false;
            }
            (Setup2, addr, 0x30) => {
                // Erase 4KB sector.
                if self.erase_mode {
                    let sector = self.address(addr & 0xF000);
                    for i in sector..(sector + 4 * 1024) {
                        file.write(i, &[0xFF]);
                    }
                }
                self.command = Ready;
                self.erase_mode = false;
            }
            (_, 0x5555, 0xF0) => {
                // Forcibly return to ready mode.
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

    /// Translate an address to the chip address given the bank.
    fn address(&self, addr: u32) -> usize {
        ((self.bank as usize) * 64 * 1024) + (addr as usize)
    }
}
