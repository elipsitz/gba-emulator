#![allow(unused)]

use crate::{io::WaitControl, Addr, Gba, Memory};

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

#[derive(Copy, Clone, Debug)]
enum MemoryAccessSize {
    Mem8 = 0,
    Mem16 = 1,
    Mem32 = 2,
}

pub const REGION_BIOS: Addr = 0x0;
pub const REGION_EWRAM: Addr = 0x2;
pub const REGION_IWRAM: Addr = 0x3;
pub const REGION_IO: Addr = 0x4;
pub const REGION_PALETTE: Addr = 0x5;
pub const REGION_VRAM: Addr = 0x6;
pub const REGION_OAM: Addr = 0x7;
pub const REGION_CART_WS0_A: Addr = 0x8;
pub const REGION_CART_WS0_B: Addr = 0x9;
pub const REGION_CART_WS1_A: Addr = 0xA;
pub const REGION_CART_WS1_B: Addr = 0xB;
pub const REGION_CART_WS2_A: Addr = 0xC;
pub const REGION_CART_WS2_B: Addr = 0xD;
pub const REGION_SRAM: Addr = 0xE;
pub const REGION_CART_UNUSED: Addr = 0xF;

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

        bus.wait_s16[REGION_VRAM as usize] = 1;
        bus.wait_n16[REGION_VRAM as usize] = 1;
        bus.wait_s32[REGION_VRAM as usize] = 2;
        bus.wait_n32[REGION_VRAM as usize] = 2;

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

        bus.update_waitcnt(WaitControl(0));
        bus
    }

    /// Update cycle timing tables after WAITCNT is updated.
    pub(crate) fn update_waitcnt(&mut self, waitcnt: WaitControl) {
        let sram = [4, 3, 2, 8][waitcnt.sram() as usize];
        let ws0_n = [4, 3, 2, 8][waitcnt.ws0_nonsequential() as usize];
        let ws0_s = [2, 1][waitcnt.ws0_sequential() as usize];
        let ws1_n = [4, 3, 2, 8][waitcnt.ws1_nonsequential() as usize];
        let ws1_s = [4, 1][waitcnt.ws1_sequential() as usize];
        let ws2_n = [4, 3, 2, 8][waitcnt.ws2_nonsequential() as usize];
        let ws2_s = [8, 1][waitcnt.ws2_sequential() as usize];
        // TODO handle prefetch buffer.

        let wait_n = [ws0_n, ws1_n, ws2_n];
        let wait_s = [ws0_s, ws1_s, ws2_s];
        for region in REGION_CART_WS0_A..=REGION_CART_WS2_B {
            let ws = ((region - REGION_CART_WS0_A) / 2) as usize;
            self.wait_n16[region as usize] = 1 + wait_n[ws];
            self.wait_s16[region as usize] = 1 + wait_s[ws];
            self.wait_n32[region as usize] = 1 + wait_n[ws] + 1 + wait_s[ws];
            self.wait_s32[region as usize] = 1 + wait_s[ws] + 1 + wait_s[ws];
        }
        for region in REGION_SRAM..=REGION_CART_UNUSED {
            self.wait_n16[region as usize] = 1 + sram;
            self.wait_s16[region as usize] = 1 + sram;
            self.wait_n32[region as usize] = 1 + sram;
            self.wait_s32[region as usize] = 1 + sram;
        }
    }
}

impl Gba {
    /// Add cycles for the memory read.
    fn add_cycles(&mut self, region: u32, size: MemoryAccessSize, access: MemoryAccessType) {
        use MemoryAccessSize::*;
        use MemoryAccessType::*;
        // TODO: OAM/Palette/VRAM have "plus 1 cycle if GBA access video mem at same time".
        let table = match (size, access) {
            (Mem8 | Mem16, Sequential) => &self.bus.wait_s16,
            (Mem8 | Mem16, NonSequential) => &self.bus.wait_n16,
            (Mem32, Sequential) => &self.bus.wait_s32,
            (Mem32, NonSequential) => &self.bus.wait_n32,
        };
        let cycles = table[(region as usize) & 0xF];
        debug_assert!(
            cycles > 0,
            "region={} size={:?} access={:?}",
            region,
            size,
            access
        );
        self.scheduler.update(cycles);
    }

    /// Read a 32 bit value from the bus.
    pub(crate) fn cpu_load32(&mut self, addr: Addr, access: MemoryAccessType) -> u32 {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem32, access);

        match region {
            // TODO only allow reading BIOS if PC is in BIOS
            REGION_BIOS => self.bios_rom.read_32(addr & 0x3FFF),
            REGION_EWRAM => self.ewram.read_32(addr & 0x3FFFF),
            REGION_IWRAM => self.iwram.read_32(addr & 0x7FFF),
            REGION_IO => self.io_read_32(addr),
            REGION_VRAM => self.ppu.vram.read_32(addr & 0x1FFFF), // TODO wrap better?
            REGION_PALETTE => self.ppu.palette.read_32(addr & 0x3FF),
            REGION_OAM => self.ppu.oam.read_32(addr & 0x3FF),
            REGION_CART_WS0_A..=REGION_CART_UNUSED => self.cartridge.read_32(addr),
            _ => {
                eprintln!("Bad memory load (32 bit) at {:X}", addr);
                0
            }
        }
    }

    /// Read a 16 bit value from the bus.
    pub(crate) fn cpu_load16(&mut self, addr: Addr, access: MemoryAccessType) -> u16 {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem16, access);

        match region {
            REGION_BIOS => self.bios_rom.read_16(addr & 0x3FFF),
            REGION_EWRAM => self.ewram.read_16(addr & 0x3FFFF),
            REGION_IWRAM => self.iwram.read_16(addr & 0x7FFF),
            REGION_IO => self.io_read_16(addr),
            REGION_VRAM => self.ppu.vram.read_16(addr & 0x1FFFF), // TODO wrap better?
            REGION_PALETTE => self.ppu.palette.read_16(addr & 0x3FF),
            REGION_OAM => self.ppu.oam.read_16(addr & 0x3FF),
            REGION_CART_WS0_A..=REGION_CART_UNUSED => self.cartridge.read_16(addr),
            _ => {
                eprintln!("Bad memory load (16 bit) at {:X}", addr);
                0
            }
        }
    }

    /// Read an 8 bit value from the bus.
    pub(crate) fn cpu_load8(&mut self, addr: Addr, access: MemoryAccessType) -> u8 {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem8, access);

        match region {
            REGION_BIOS => self.bios_rom.read_8(addr & 0x3FFF),
            REGION_EWRAM => self.ewram.read_8(addr & 0x3FFFF),
            REGION_IWRAM => self.iwram.read_8(addr & 0x7FFF),
            REGION_IO => self.io_read_8(addr),
            REGION_VRAM => self.ppu.vram.read_8(addr & 0x1FFFF), // TODO wrap better?
            REGION_PALETTE => self.ppu.palette.read_8(addr & 0x3FF),
            REGION_OAM => self.ppu.oam.read_8(addr & 0x3FF),
            REGION_CART_WS0_A..=REGION_CART_UNUSED => self.cartridge.read_8(addr),
            _ => {
                eprintln!("Bad memory load (8 bit) at {:X}", addr);
                0
            }
        }
    }

    /// Store a 32 bit value to the bus.
    pub(crate) fn cpu_store32(&mut self, addr: Addr, data: u32, access: MemoryAccessType) {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem32, access);

        match region {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_32(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_32(addr & 0x7FFF, data),
            REGION_IO => self.io_write_32(addr, data),
            REGION_VRAM => self.ppu.vram.write_32(addr & 0x1FFFF, data), // TODO wrap better?
            REGION_PALETTE => self.ppu.palette.write_32(addr & 0x3FF, data),
            REGION_OAM => self.ppu.oam.write_32(addr & 0x3FF, data),
            REGION_CART_WS0_A..=REGION_CART_UNUSED => self.cartridge.write_32(addr, data),
            _ => {
                eprintln!(
                    "Bad memory store (32 bit) at {:X}, data {:X}, PC={:08X}",
                    addr, data, self.cpu.pc
                );
            }
        }
    }

    /// Store a 16 bit value to the bus.
    pub(crate) fn cpu_store16(&mut self, addr: Addr, data: u16, access: MemoryAccessType) {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem16, access);

        match region {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_16(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_16(addr & 0x7FFF, data),
            REGION_IO => self.io_write_16(addr, data),
            REGION_VRAM => self.ppu.vram.write_16(addr & 0x1FFFF, data), // TODO wrap better?
            REGION_PALETTE => self.ppu.palette.write_16(addr & 0x3FF, data),
            REGION_OAM => self.ppu.oam.write_16(addr & 0x3FF, data),
            REGION_CART_WS0_A..=REGION_CART_UNUSED => self.cartridge.write_16(addr, data),
            _ => {
                eprintln!("Bad memory store (16 bit) at {:X}, data {:X}", addr, data);
            }
        }
    }

    /// Store an 8 bit value to the bus.
    pub(crate) fn cpu_store8(&mut self, addr: Addr, data: u8, access: MemoryAccessType) {
        let region = region_from_address(addr);
        self.add_cycles(region, MemoryAccessSize::Mem8, access);

        match region {
            REGION_BIOS => {}
            REGION_EWRAM => self.ewram.write_8(addr & 0x3FFFF, data),
            REGION_IWRAM => self.iwram.write_8(addr & 0x7FFF, data),
            REGION_IO => self.io_write_8(addr, data),
            REGION_VRAM => self.ppu.vram.write_8(addr & 0x1FFFF, data), // TODO wrap better?
            REGION_PALETTE => self.ppu.palette.write_8(addr & 0x3FF, data),
            REGION_OAM => self.ppu.oam.write_8(addr & 0x3FF, data),
            REGION_CART_WS0_A..=REGION_CART_UNUSED => self.cartridge.write_8(addr, data),
            _ => {
                eprintln!("Bad memory store (8 bit) at {:X}, data {:X}", addr, data);
            }
        }
    }
}
