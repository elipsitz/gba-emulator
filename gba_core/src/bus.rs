#![allow(unused)]

use crate::{Addr, Gba, Memory};

/// Memory access types.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MemoryAccessType {
    Sequential,
    NonSequential,
}

const REGION_BIOS: Addr = 0x00000000;
const REGION_EWRAM: Addr = 0x02000000;
const REGION_IWRAM: Addr = 0x03000000;
const REGION_IO: Addr = 0x04000000;
const REGION_PALETTE: Addr = 0x05000000;
const REGION_VRAM: Addr = 0x06000000;
const REGION_OAM: Addr = 0x07000000;
const REGION_CART_WS0_A: Addr = 0x08000000;
const REGION_CART_WS0_B: Addr = 0x09000000;
const REGION_CART_WS1_A: Addr = 0x0A000000;
const REGION_CART_WS1_B: Addr = 0x0B000000;
const REGION_CART_WS2_A: Addr = 0x0C000000;
const REGION_CART_WS2_B: Addr = 0x0D000000;
const REGION_SRAM: Addr = 0x0E000000;

impl Gba {
    /// Read a 32 bit value from the bus.
    pub fn cpu_load32(&mut self, addr: Addr, _access: MemoryAccessType) -> u32 {
        // TODO increment cycles properly
        match addr & 0xFF000000 {
            // TODO only allow reading BIOS if PC is in BIOS
            REGION_BIOS => self.bios_rom.read_32(addr & 0x3FFF),
            REGION_EWRAM => self.ewram.read_32(addr & 0x3FFFF),
            REGION_IWRAM => self.iwram.read_32(addr & 0x7FFF),
            REGION_CART_WS0_A | REGION_CART_WS0_B | REGION_CART_WS1_A | REGION_CART_WS1_B
            | REGION_CART_WS2_A | REGION_CART_WS2_B => {
                let data = &mut self.cart_rom.data;
                let addr = addr & 0x1FFFFFF;
                if (addr as usize) < data.len() {
                    data.read_32(addr)
                } else {
                    0
                }
            }
            _ => {
                eprintln!("Bad memory load (32 bit) at {:X}", addr);
                0
            }
        }
    }

    /// Read a 16 bit value from the bus.
    pub fn cpu_load16(&mut self, addr: Addr, _access: MemoryAccessType) -> u16 {
        // TODO increment cycles properly
        match addr & 0xFF000000 {
            REGION_BIOS => self.bios_rom.read_16(addr & 0x3FFF),
            REGION_EWRAM => self.ewram.read_16(addr & 0x3FFFF),
            REGION_IWRAM => self.iwram.read_16(addr & 0x7FFF),
            REGION_CART_WS0_A | REGION_CART_WS0_B | REGION_CART_WS1_A | REGION_CART_WS1_B
            | REGION_CART_WS2_A | REGION_CART_WS2_B => {
                let data = &mut self.cart_rom.data;
                let addr = addr & 0x1FFFFFF;
                if (addr as usize) < data.len() {
                    data.read_16(addr)
                } else {
                    0
                }
            }
            _ => {
                eprintln!("Bad memory load (16 bit) at {:X}", addr);
                0
            }
        }
    }

    /// Read an 8 bit value from the bus.
    pub fn cpu_load8(&mut self, addr: Addr, _access: MemoryAccessType) -> u8 {
        // TODO increment cycles properly
        match addr & 0xFF000000 {
            REGION_BIOS => self.bios_rom.read_8(addr & 0x3FFF),
            REGION_EWRAM => self.ewram.read_8(addr & 0x3FFFF),
            REGION_IWRAM => self.iwram.read_8(addr & 0x7FFF),
            REGION_CART_WS0_A | REGION_CART_WS0_B | REGION_CART_WS1_A | REGION_CART_WS1_B
            | REGION_CART_WS2_A | REGION_CART_WS2_B => {
                let data = &mut self.cart_rom.data;
                let addr = addr & 0x1FFFFFF;
                if (addr as usize) < data.len() {
                    data.read_8(addr)
                } else {
                    0
                }
            }
            _ => {
                eprintln!("Bad memory load (8 bit) at {:X}", addr);
                0
            }
        }
    }

    /// Store a 32 bit value to the bus.
    pub fn cpu_store32(&mut self, addr: Addr, data: u32, _access: MemoryAccessType) {
        // TODO increment cycles properly
        match addr & 0xFF000000 {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_32(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_32(addr & 0x7FFF, data),
            _ => {
                eprintln!("Bad memory store (32 bit) at {:X}", addr);
            }
        }
    }

    /// Store a 16 bit value to the bus.
    pub fn cpu_store16(&mut self, addr: Addr, data: u16, _access: MemoryAccessType) {
        // TODO increment cycles properly
        match addr & 0xFF000000 {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_16(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_16(addr & 0x7FFF, data),
            _ => {
                eprintln!("Bad memory store (32 bit) at {:X}", addr);
            }
        }
    }

    /// Store an 8 bit value to the bus.
    pub fn cpu_store8(&mut self, addr: Addr, data: u8, _access: MemoryAccessType) {
        // TODO increment cycles properly
        match addr & 0xFF000000 {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_8(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_8(addr & 0x7FFF, data),
            _ => {
                eprintln!("Bad memory store (32 bit) at {:X}", addr);
            }
        }
    }
}
