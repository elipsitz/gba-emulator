#![allow(unused)]

use crate::{Addr, Gba, Memory};

/// State for the system memory bus.
pub struct Bus {
    wait_s16: [usize; 16],
    wait_n16: [usize; 16],
    wait_s32: [usize; 16],
    wait_n32: [usize; 16],
}

/// Memory access types.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MemoryAccessType {
    Sequential,
    NonSequential,
}

#[derive(Copy, Clone)]
enum MemoryAccessSize {
    Mem8 = 0,
    Mem16 = 1,
    Mem32 = 2,
}

const REGION_BIOS: Addr = 0x0;
const REGION_EWRAM: Addr = 0x2;
const REGION_IWRAM: Addr = 0x3;
const REGION_IO: Addr = 0x4;
const REGION_PALETTE: Addr = 0x5;
const REGION_VRAM: Addr = 0x6;
const REGION_OAM: Addr = 0x7;
const REGION_CART_WS0_A: Addr = 0x8;
const REGION_CART_WS0_B: Addr = 0x9;
const REGION_CART_WS1_A: Addr = 0xA;
const REGION_CART_WS1_B: Addr = 0xB;
const REGION_CART_WS2_A: Addr = 0xC;
const REGION_CART_WS2_B: Addr = 0xD;
const REGION_SRAM: Addr = 0xE;

/// Address to region.
#[inline(always)]
pub fn region_from_address(addr: Addr) -> u32 {
    (addr & 0x0F00_0000) >> 24
}

impl Bus {
    /// New bus in the initial state.
    pub fn new() -> Bus {
        let mut bus = Bus {
            wait_s16: [0; 16],
            wait_n16: [0; 16],
            wait_s32: [0; 16],
            wait_n32: [0; 16],
        };

        bus.wait_s16[REGION_BIOS as usize] = 1;
        bus.wait_n16[REGION_BIOS as usize] = 1;
        bus.wait_s32[REGION_BIOS as usize] = 1;
        bus.wait_n32[REGION_BIOS as usize] = 1;

        bus.wait_s16[REGION_IWRAM as usize] = 1;
        bus.wait_n16[REGION_IWRAM as usize] = 1;
        bus.wait_s32[REGION_IWRAM as usize] = 1;
        bus.wait_n32[REGION_IWRAM as usize] = 1;

        bus.wait_s16[REGION_IO as usize] = 1;
        bus.wait_n16[REGION_IO as usize] = 1;
        bus.wait_s32[REGION_IO as usize] = 1;
        bus.wait_n32[REGION_IO as usize] = 1;

        bus.wait_s16[REGION_OAM as usize] = 1;
        bus.wait_n16[REGION_OAM as usize] = 1;
        bus.wait_s32[REGION_OAM as usize] = 1;
        bus.wait_n32[REGION_OAM as usize] = 1;

        bus.wait_s16[REGION_EWRAM as usize] = 3;
        bus.wait_n16[REGION_EWRAM as usize] = 3;
        bus.wait_s32[REGION_EWRAM as usize] = 6;
        bus.wait_n32[REGION_EWRAM as usize] = 6;

        bus.wait_s16[REGION_PALETTE as usize] = 1;
        bus.wait_n16[REGION_PALETTE as usize] = 1;
        bus.wait_s32[REGION_PALETTE as usize] = 2;
        bus.wait_n32[REGION_PALETTE as usize] = 2;

        bus.wait_s16[REGION_CART_WS0_A as usize] = 3;
        bus.wait_n16[REGION_CART_WS0_A as usize] = 5;
        bus.wait_s32[REGION_CART_WS0_A as usize] = 6;
        bus.wait_n32[REGION_CART_WS0_A as usize] = 8;

        bus.wait_s16[REGION_CART_WS0_B as usize] = 3;
        bus.wait_n16[REGION_CART_WS0_B as usize] = 5;
        bus.wait_s32[REGION_CART_WS0_B as usize] = 6;
        bus.wait_n32[REGION_CART_WS0_B as usize] = 8;

        bus.wait_s16[REGION_CART_WS1_A as usize] = 5;
        bus.wait_n16[REGION_CART_WS1_A as usize] = 5;
        bus.wait_s32[REGION_CART_WS1_A as usize] = 10;
        bus.wait_n32[REGION_CART_WS1_A as usize] = 10;

        bus.wait_s16[REGION_CART_WS1_B as usize] = 5;
        bus.wait_n16[REGION_CART_WS1_B as usize] = 5;
        bus.wait_s32[REGION_CART_WS1_B as usize] = 10;
        bus.wait_n32[REGION_CART_WS1_B as usize] = 10;

        bus.wait_s16[REGION_CART_WS2_A as usize] = 9;
        bus.wait_n16[REGION_CART_WS2_A as usize] = 5;
        bus.wait_s32[REGION_CART_WS2_A as usize] = 18;
        bus.wait_n32[REGION_CART_WS2_A as usize] = 14;

        bus.wait_s16[REGION_CART_WS2_B as usize] = 9;
        bus.wait_n16[REGION_CART_WS2_B as usize] = 5;
        bus.wait_s32[REGION_CART_WS2_B as usize] = 18;
        bus.wait_n32[REGION_CART_WS2_B as usize] = 14;

        bus.wait_s16[REGION_SRAM as usize] = 5;
        bus.wait_n16[REGION_SRAM as usize] = 5;
        bus.wait_s32[REGION_SRAM as usize] = 5;
        bus.wait_n32[REGION_SRAM as usize] = 5;

        bus
    }
}

impl Gba {
    /// Add cycles for the memory read.
    fn add_cycles(&mut self, region: u32, size: MemoryAccessSize, access: MemoryAccessType) {
        use MemoryAccessSize::*;
        use MemoryAccessType::*;
        // TODO: handle switching waitstates
        // TODO: OAM/Palette/VRAM have "plus 1 cycle if GBA access video mem at same time".
        let table = match (size, access) {
            (Mem8 | Mem16, Sequential) => &self.bus.wait_s16,
            (Mem8 | Mem16, NonSequential) => &self.bus.wait_n16,
            (Mem32, Sequential) => &self.bus.wait_s32,
            (Mem32, NonSequential) => &self.bus.wait_n32,
        };
        let cycles = table[(region as usize) & 0xF];
        self.scheduler.update(cycles);
    }

    /// Read a 32 bit value from the bus.
    pub fn cpu_load32(&mut self, addr: Addr, access: MemoryAccessType) -> u32 {
        // TODO: remove gbatest failure detection
        if addr == 0x0400_0004 {
            eprintln!("{}", self.cpu_format_debug());
            panic!("cputest failed! see r12");
        }

        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem32, access);

        match region {
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
    pub fn cpu_load16(&mut self, addr: Addr, access: MemoryAccessType) -> u16 {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem16, access);

        match region {
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
    pub fn cpu_load8(&mut self, addr: Addr, access: MemoryAccessType) -> u8 {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem8, access);

        match region {
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
    pub fn cpu_store32(&mut self, addr: Addr, data: u32, access: MemoryAccessType) {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem32, access);

        match region {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_32(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_32(addr & 0x7FFF, data),
            _ => {
                eprintln!("Bad memory store (32 bit) at {:X}", addr);
            }
        }
    }

    /// Store a 16 bit value to the bus.
    pub fn cpu_store16(&mut self, addr: Addr, data: u16, access: MemoryAccessType) {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem16, access);

        match region {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_16(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_16(addr & 0x7FFF, data),
            _ => {
                eprintln!("Bad memory store (32 bit) at {:X}", addr);
            }
        }
    }

    /// Store an 8 bit value to the bus.
    pub fn cpu_store8(&mut self, addr: Addr, data: u8, access: MemoryAccessType) {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem8, access);

        match region {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_8(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_8(addr & 0x7FFF, data),
            _ => {
                eprintln!("Bad memory store (32 bit) at {:X}", addr);
            }
        }
    }
}
