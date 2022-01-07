use bit::BitIndex;

use crate::Gba;

/// State for memory mapped IO controller.
pub struct Io {
    /// Value of the KEYCNT (keypad control) register.
    pub keycnt: u16,
    /// Current CPU power state.
    pub power_state: CpuPowerState,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum CpuPowerState {
    /// Regular power state, running as usual.
    Normal,
    /// Halted, waiting for an interrupt to occur.
    Halted,
    /// Stopped, super low power state.
    Stopped,
}

impl Io {
    pub fn new() -> Io {
        Io {
            keycnt: 0,
            power_state: CpuPowerState::Normal,
        }
    }
}

impl Gba {
    pub fn io_read_16(&mut self, addr: u32) -> u16 {
        match addr {
            REG_DISPCNT => self.ppu.dispcnt.read(),
            REG_DISPSTAT => self.ppu.dispstat.read(),
            REG_VCOUNT => self.ppu.vcount as u16,
            REG_KEYINPUT => self.keypad_state.into(),
            REG_KEYCNT => self.io.keycnt,
            REG_BG0CNT => self.ppu.bgcnt[0].read(),
            REG_BG1CNT => self.ppu.bgcnt[1].read(),
            REG_BG2CNT => self.ppu.bgcnt[2].read(),
            REG_BG3CNT => self.ppu.bgcnt[3].read(),
            REG_IME => self.interrupt.global_enabled as u16,
            REG_IE => self.interrupt.enabled,
            REG_IF => self.interrupt.pending,
            REG_DMA_START..=REG_DMA_END => self.dma_reg_read(addr - REG_DMA_START),
            _ => 0,
        }
    }

    pub fn io_write_16(&mut self, addr: u32, value: u16) {
        match addr {
            REG_DISPCNT => self.ppu.dispcnt.write(value),
            REG_DISPSTAT => self.ppu.dispstat.write(value),
            REG_KEYCNT => self.io.keycnt = value,
            REG_BG0CNT => self.ppu.bgcnt[0].write(value),
            REG_BG1CNT => self.ppu.bgcnt[1].write(value),
            REG_BG2CNT => self.ppu.bgcnt[2].write(value),
            REG_BG3CNT => self.ppu.bgcnt[3].write(value),
            REG_BG0HOFS => self.ppu.bg_hofs[0] = value & 0x1FF,
            REG_BG0VOFS => self.ppu.bg_vofs[0] = value & 0x1FF,
            REG_BG1HOFS => self.ppu.bg_hofs[1] = value & 0x1FF,
            REG_BG1VOFS => self.ppu.bg_vofs[1] = value & 0x1FF,
            REG_BG2HOFS => self.ppu.bg_hofs[2] = value & 0x1FF,
            REG_BG2VOFS => self.ppu.bg_vofs[2] = value & 0x1FF,
            REG_BG3HOFS => self.ppu.bg_hofs[3] = value & 0x1FF,
            REG_BG3VOFS => self.ppu.bg_vofs[3] = value & 0x1FF,
            REG_BG2PA => self.ppu.bg_affine[0].pa = value as i16,
            REG_BG2PB => self.ppu.bg_affine[0].pb = value as i16,
            REG_BG2PC => self.ppu.bg_affine[0].pc = value as i16,
            REG_BG2PD => self.ppu.bg_affine[0].pd = value as i16,
            REG_BG3PA => self.ppu.bg_affine[1].pa = value as i16,
            REG_BG3PB => self.ppu.bg_affine[1].pb = value as i16,
            REG_BG3PC => self.ppu.bg_affine[1].pc = value as i16,
            REG_BG3PD => self.ppu.bg_affine[1].pd = value as i16,
            REG_BG2X_L => {
                set_reg_displacement_lo(&mut self.ppu.bg_affine[0].dx, value);
                self.ppu.bg_affine[0].internal_dx = self.ppu.bg_affine[0].dx;
            }
            REG_BG2X_H => {
                set_reg_displacement_hi(&mut self.ppu.bg_affine[0].dx, value);
                self.ppu.bg_affine[0].internal_dx = self.ppu.bg_affine[0].dx;
            }
            REG_BG2Y_L => {
                set_reg_displacement_lo(&mut self.ppu.bg_affine[0].dy, value);
                self.ppu.bg_affine[0].internal_dy = self.ppu.bg_affine[0].dy;
            }
            REG_BG2Y_H => {
                set_reg_displacement_hi(&mut self.ppu.bg_affine[0].dy, value);
                self.ppu.bg_affine[0].internal_dy = self.ppu.bg_affine[0].dy;
            }
            REG_BG3X_L => {
                set_reg_displacement_lo(&mut self.ppu.bg_affine[1].dx, value);
                self.ppu.bg_affine[1].internal_dx = self.ppu.bg_affine[1].dx;
            }
            REG_BG3X_H => {
                set_reg_displacement_hi(&mut self.ppu.bg_affine[1].dx, value);
                self.ppu.bg_affine[1].internal_dx = self.ppu.bg_affine[1].dx;
            }
            REG_BG3Y_L => {
                set_reg_displacement_lo(&mut self.ppu.bg_affine[1].dy, value);
                self.ppu.bg_affine[1].internal_dy = self.ppu.bg_affine[1].dy;
            }
            REG_BG3Y_H => {
                set_reg_displacement_hi(&mut self.ppu.bg_affine[1].dy, value);
                self.ppu.bg_affine[1].internal_dy = self.ppu.bg_affine[1].dy;
            }
            REG_MOSAIC => self.ppu.mosaic.write(value),
            REG_IME => self.interrupt.global_enabled = value & 1 == 1,
            REG_IE => self.interrupt.enabled = value & 0x3FFF,
            REG_IF => self.interrupt_reg_if_write(value),
            REG_DMA_START..=REG_DMA_END => self.dma_reg_write(addr - REG_DMA_START, value),
            _ => {}
        }
    }

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
        match addr {
            REG_HALTCNT => {
                if value.bit(7) {
                    self.io.power_state = CpuPowerState::Stopped;
                    todo!("HALTCNT = STOP not supported");
                } else {
                    self.io.power_state = CpuPowerState::Halted;
                }
            }
            _ => {
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
    }
}

/// Set the low 16-bits of a 32-bit affine background displacement register.
fn set_reg_displacement_lo(register: &mut i32, value: u16) {
    let old = (*register) as u32;
    let new = (old & 0xFFFF_0000) | (value as u32);
    *register = new as i32;
}

/// Set the high 16-bits of a 32-bit affine background displacement register.
fn set_reg_displacement_hi(register: &mut i32, value: u16) {
    // Only use 12 bits (sign extend the upper 4 bits).
    let value = ((value as u32) & 0x0FFF) << 16;
    let value = if value.bit(32 - 4 - 1) {
        value | 0xF000_0000
    } else {
        value
    };
    let old = (*register) as u32;
    let new = (old & 0x0000_FFFF) | value;
    *register = new as i32;
}

pub const REG_DISPCNT: u32 = 0x0400_0000;
pub const REG_DISPSTAT: u32 = 0x0400_0004;
pub const REG_VCOUNT: u32 = 0x0400_0006;
pub const REG_KEYINPUT: u32 = 0x0400_0130;
pub const REG_KEYCNT: u32 = 0x0400_0132;
pub const REG_BG0CNT: u32 = 0x0400_0008;
pub const REG_BG1CNT: u32 = 0x0400_000A;
pub const REG_BG2CNT: u32 = 0x0400_000C;
pub const REG_BG3CNT: u32 = 0x0400_000E;
pub const REG_BG0HOFS: u32 = 0x0400_0010;
pub const REG_BG0VOFS: u32 = 0x0400_0012;
pub const REG_BG1HOFS: u32 = 0x0400_0014;
pub const REG_BG1VOFS: u32 = 0x0400_0016;
pub const REG_BG2HOFS: u32 = 0x0400_0018;
pub const REG_BG2VOFS: u32 = 0x0400_001A;
pub const REG_BG3HOFS: u32 = 0x0400_001C;
pub const REG_BG3VOFS: u32 = 0x0400_001E;

pub const REG_BG2PA: u32 = 0x0400_0020;
pub const REG_BG2PB: u32 = 0x0400_0022;
pub const REG_BG2PC: u32 = 0x0400_0024;
pub const REG_BG2PD: u32 = 0x0400_0026;
pub const REG_BG2X_L: u32 = 0x0400_0028;
pub const REG_BG2X_H: u32 = 0x0400_002A;
pub const REG_BG2Y_L: u32 = 0x0400_002C;
pub const REG_BG2Y_H: u32 = 0x0400_002E;

pub const REG_BG3PA: u32 = 0x0400_0030;
pub const REG_BG3PB: u32 = 0x0400_0032;
pub const REG_BG3PC: u32 = 0x0400_0034;
pub const REG_BG3PD: u32 = 0x0400_0036;
pub const REG_BG3X_L: u32 = 0x0400_0038;
pub const REG_BG3X_H: u32 = 0x0400_003A;
pub const REG_BG3Y_L: u32 = 0x0400_003C;
pub const REG_BG3Y_H: u32 = 0x0400_003E;

pub const REG_MOSAIC: u32 = 0x0400_004C;

pub const REG_IME: u32 = 0x0400_0208;
pub const REG_IE: u32 = 0x0400_0200;
pub const REG_IF: u32 = 0x0400_0202;
pub const REG_HALTCNT: u32 = 0x0400_0301;

pub const REG_DMA_START: u32 = 0x0400_00B0;
pub const REG_DMA_END: u32 = 0x0400_00DE;
