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
            REG_IME => self.interrupt.global_enabled = value & 1 == 1,
            REG_IE => self.interrupt.enabled = value & 0x3FFF,
            REG_IF => self.interrupt_reg_if_write(value),
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
pub const REG_IME: u32 = 0x0400_0208;
pub const REG_IE: u32 = 0x0400_0200;
pub const REG_IF: u32 = 0x0400_0202;
pub const REG_HALTCNT: u32 = 0x0400_0301;
