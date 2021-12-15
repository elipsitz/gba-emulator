pub type Addr = u32;

/// Trait for something that bytes can be read from and/or written to.
pub trait Memory {
    /// Read 8 bits.
    fn read_8(&mut self, addr: Addr) -> u8;

    /// Write 8 bits.
    fn write_8(&mut self, addr: Addr, value: u8);

    fn read_16(&mut self, addr: Addr) -> u16 {
        (self.read_8(addr) as u16) | ((self.read_8(addr + 1) as u16) << 8)
    }

    fn write_16(&mut self, addr: Addr, value: u16) {
        self.write_8(addr, (value & 0xFF) as u8);
        self.write_8(addr + 1, ((value >> 8) & 0xFF) as u8);
    }

    fn read_32(&mut self, addr: Addr) -> u32 {
        (self.read_16(addr) as u32) | ((self.read_16(addr + 2) as u32) << 16)
    }

    fn write_32(&mut self, addr: Addr, value: u32) {
        self.write_16(addr, (value & 0xFFFF) as u16);
        self.write_16(addr + 2, ((value >> 16) & 0xFFFF) as u16);
    }
}

impl Memory for [u8] {
    fn read_8(&mut self, addr: Addr) -> u8 {
        self[addr as usize]
    }

    fn write_8(&mut self, addr: Addr, value: u8) {
        self[addr as usize] = value;
    }
}
