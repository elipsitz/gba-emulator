use crate::Gba;

/// State for memory mapped IO controller.
pub struct Io {}

impl Io {
    pub fn new() -> Io {
        Io {}
    }
}

impl Gba {
    pub fn io_read_16(&mut self, addr: u32) -> u16 {
        0
    }

    pub fn io_write_16(&mut self, addr: u32, value: u16) {}

    pub fn io_read_32(&mut self, addr: u32) -> u32 {
        (self.io_read_16(addr) as u32) | ((self.io_read_16(addr + 2) as u32) << 16)
    }

    pub fn io_write_32(&mut self, addr: u32, value: u32) {
        self.io_write_16(addr, (value & 0xFFFF) as u16);
        self.io_write_16(addr + 2, ((value >> 16) & 0xFFFF) as u16);
    }

    pub fn io_read_8(&mut self, addr: u32) -> u8 {
        let value = self.io_read_16(addr & !1);
        if addr & 1 == 0 {
            value as u8
        } else {
            (value >> 8) as u8
        }
    }

    pub fn io_write_8(&mut self, addr: u32, value: u8) {
        let full = self.io_read_16(addr & !1);
        let writeback = if addr & 1 == 0 {
            // Replace the low byte.
            (full & 0xFF00) | (value as u16)
        } else {
            (full & 0x00FF) | ((value as u16) << 8)
        };
        self.io_write_16(addr & !1, writeback);
    }
}
