use bit::BitIndex;
use serde::{Deserialize, Serialize};

use crate::Dma;

use super::BackupBuffer;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum EepromSize {
    /// 512 byte EEPROM.
    Eeprom512,
    /// 8 KB EEPROM.
    Eeprom8K,
}

impl EepromSize {
    /// Return the length of the address for this EEPROM size in bits.
    fn address_bits(self) -> usize {
        match self {
            EepromSize::Eeprom512 => 6,
            EepromSize::Eeprom8K => 14,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct EepromBackup {
    /// Size of the EEPROM, or None if it's pending autodetection.
    size: Option<EepromSize>,

    /// Serial buffer.
    serial_buffer: u64,
    /// How many bits have been transferred.
    serial_transferred: usize,
    /// Number of bits read in a Read operation.
    read_bits: usize,

    /// Current command state.
    state: State,
    /// Whether we're processing a read request or a write request.
    request_type: RequestType,
    /// Current address we're reading / writing.
    address: usize,
}

#[derive(Copy, Clone, PartialEq, Debug, Serialize, Deserialize)]
enum State {
    /// Waiting for a command.
    Waiting,
    /// Getting an address.
    GetAddress,
    /// Handling the extra bit at the end of a read address.
    ReadAddressSuffix,
    /// Handling the extra bit at the end of a write.
    WriteSuffix,
    /// Reading data from EEPROM.
    Reading,
    /// Writing data to EEPROM.
    Writing,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
enum RequestType {
    Read,
    Write,
}

impl EepromBackup {
    pub fn new(size: Option<EepromSize>) -> EepromBackup {
        EepromBackup {
            size,
            serial_buffer: 0,
            serial_transferred: 0,
            state: State::Waiting,
            request_type: RequestType::Read,
            address: 0,
            read_bits: 0,
        }
    }

    /// Read a bit from EEPROM.
    pub fn read(&mut self, buffer: &mut BackupBuffer) -> u16 {
        if self.state == State::Reading {
            self.read_bits += 1;
            if self.read_bits <= 4 {
                // First four bits are zeros.
                0
            } else {
                if self.read_bits == 64 + 4 {
                    // After this, we're done reading.
                    self.state = State::Waiting;
                    self.reset_serial();
                }

                // We write the bits such that 0 is the MSB, 7 is the LSB.
                let bit = self.read_bits - 5;
                let address = self.address + (bit / 8);
                let bit_index = 7 - (bit % 8);

                let value = buffer.read(address);
                value.bit(bit_index) as u16
            }
        } else {
            1
        }
    }

    /// Reset the serial buffer.
    fn reset_serial(&mut self) {
        self.serial_buffer = 0;
        self.serial_transferred = 0;
    }

    /// Write a bit to EEPROM.
    ///
    /// Takes a reference to the DMA engine state so that we can attempt
    /// to autodetect EEPROM size based on the DMA transfer count.
    pub fn write(&mut self, value: u16, dma: &Dma, buffer: &mut BackupBuffer) {
        let size = self.size.get_or_insert_with(|| {
            // Try to detect the size of the EEPROM.
            match Self::detect_size(dma) {
                Some(size) => {
                    eprintln!("EEPROM: detected {:?}", size);
                    size
                }
                None => panic!("Failed to detect EEPROM size!"),
            }
        });

        let input = (value & 0x1) as u64;
        self.serial_buffer = self.serial_buffer.wrapping_shl(1);
        self.serial_buffer |= input;
        self.serial_transferred += 1;

        match self.state {
            State::Reading => {}
            State::Waiting if self.serial_transferred == 2 => {
                match self.serial_buffer {
                    0b11 => {
                        // Read request.
                        self.request_type = RequestType::Read;
                        self.state = State::GetAddress;
                    }
                    0b10 => {
                        // Write request.
                        self.request_type = RequestType::Write;
                        self.state = State::GetAddress;
                    }
                    _ => {}
                }
                self.reset_serial();
            }
            State::Waiting => {}
            State::GetAddress => {
                if self.serial_transferred == size.address_bits() {
                    // Addressing is in chunks of 8 bytes.
                    // For the 8KB one, we ignore the top 4 bits of the address.
                    self.address = ((self.serial_buffer as usize) * 8) & 0x1FFF;
                    self.state = match self.request_type {
                        RequestType::Read => State::ReadAddressSuffix,
                        RequestType::Write => State::Writing,
                    };
                    self.reset_serial();
                }
            }
            State::ReadAddressSuffix => {
                self.state = State::Reading;
                self.read_bits = 0;
                self.reset_serial();
            }
            State::Writing => {
                // We write the bits such that 0 is the MSB, 7 is the LSB.
                let bit = self.serial_transferred - 1;
                let address = self.address + (bit / 8);
                let bit_index = 7 - (bit % 8);

                // Write the bit.
                let mut data = buffer.read(address);
                data.set_bit(bit_index, input == 1);
                buffer.write(address, data);

                if self.serial_transferred == 64 {
                    // We're done, read the extra bit.
                    self.state = State::WriteSuffix;
                }
            }
            State::WriteSuffix => {
                self.state = State::Waiting;
                self.reset_serial();
            }
        }
    }

    /// Attempt to detect the size of EEPROM based on DMA 3's current transfer count.
    fn detect_size(dma: &crate::Dma) -> Option<EepromSize> {
        match dma.transfer_size(3) {
            // Read request for 512B: 2 + 6 + 1
            Some(9) => Some(EepromSize::Eeprom512),
            // Read request for 8KB: 2 + 14 + 1.
            Some(17) => Some(EepromSize::Eeprom8K),
            // Write request for 512B: 2 + 6 + 64 + 1.
            Some(73) => Some(EepromSize::Eeprom512),
            // Write request for 8KB: 2 + 14 + 64 + 1.
            Some(81) => Some(EepromSize::Eeprom8K),

            // Unknown size, or DMA 3 isn't active.
            _ => None,
        }
    }
}
