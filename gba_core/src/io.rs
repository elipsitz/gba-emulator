use bit::BitIndex;

use crate::Gba;

/// State for memory mapped IO controller.
pub struct Io {
    /// Value of the KEYCNT (keypad control) register.
    pub keycnt: u16,
    /// Current CPU power state.
    pub power_state: CpuPowerState,
    /// Value of the WAITCNT (wait control) register.
    pub waitcnt: WaitControl,
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
            waitcnt: WaitControl(0),
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
            REG_BLDCNT => self.ppu.bldcnt.read(),
            REG_BLDALPHA => self.ppu.bldalpha.read(),
            REG_WININ => self.ppu.win_in.read(),
            REG_WINOUT => self.ppu.win_out.read(),
            REG_TM0CNT_L | REG_TM1CNT_L | REG_TM2CNT_L | REG_TM3CNT_L => {
                self.timer_read_counter(((addr & 0b1100) >> 2) as usize)
            }
            REG_TM0CNT_H | REG_TM1CNT_H | REG_TM2CNT_H | REG_TM3CNT_H => {
                self.timer_read_control(((addr & 0b1100) >> 2) as usize)
            }
            REG_IME => self.interrupt.global_enabled as u16,
            REG_IE => self.interrupt.enabled,
            REG_IF => self.interrupt.pending,
            REG_DMA_START..=REG_DMA_END => self.dma_reg_read(addr - REG_DMA_START),
            REG_WAITCNT => self.io.waitcnt.0,
            REG_SOUND_START..=REG_SOUND_END => {
                let lo = self.apu_io_read(addr);
                let hi = self.apu_io_read(addr + 1);
                (lo as u16) | ((hi as u16) << 8)
            }
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
            REG_WIN0H => self.ppu.win_h[0].write(value),
            REG_WIN1H => self.ppu.win_h[1].write(value),
            REG_WIN0V => self.ppu.win_v[0].write(value),
            REG_WIN1V => self.ppu.win_v[1].write(value),
            REG_WININ => self.ppu.win_in.write(value),
            REG_WINOUT => self.ppu.win_out.write(value),
            REG_MOSAIC => self.ppu.mosaic.write(value),
            REG_BLDCNT => self.ppu.bldcnt.write(value),
            REG_BLDALPHA => self.ppu.bldalpha.write(value),
            REG_BLDY => self.ppu.bldy.write(value),
            REG_TM0CNT_L | REG_TM1CNT_L | REG_TM2CNT_L | REG_TM3CNT_L => {
                self.timer_write_counter(((addr & 0b1100) >> 2) as usize, value);
            }
            REG_TM0CNT_H | REG_TM1CNT_H | REG_TM2CNT_H | REG_TM3CNT_H => {
                self.timer_write_control(((addr & 0b1100) >> 2) as usize, value);
            }
            REG_IME => self.interrupt.global_enabled = value & 1 == 1,
            REG_IE => self.interrupt.enabled = value & 0x3FFF,
            REG_IF => self.interrupt_reg_if_write(value),
            REG_DMA_START..=REG_DMA_END => self.dma_reg_write(addr - REG_DMA_START, value),
            REG_WAITCNT => {
                self.io.waitcnt.0 = value & 0x7FFF;
                self.bus.update_waitcnt(self.io.waitcnt);
            }
            REG_SOUND_START..=REG_SOUND_END => {
                self.apu_io_write(addr, value as u8);
                self.apu_io_write(addr + 1, (value >> 8) as u8);
            }
            _ => {}
        }
    }

    pub fn io_read_32(&mut self, addr: u32) -> u32 {
        (self.io_read_16(addr) as u32) | ((self.io_read_16(addr + 2) as u32) << 16)
    }

    pub fn io_write_32(&mut self, addr: u32, value: u32) {
        match addr {
            REG_FIFO_A => self.apu_io_fifo_write(0, value),
            REG_FIFO_B => self.apu_io_fifo_write(1, value),
            _ => {
                self.io_write_16(addr, (value & 0xFFFF) as u16);
                self.io_write_16(addr + 2, ((value >> 16) & 0xFFFF) as u16);
            }
        }
    }

    pub fn io_read_8(&mut self, addr: u32) -> u8 {
        match addr {
            REG_SOUND_START..=REG_SOUND_END => self.apu_io_read(addr),
            _ => {
                let value = self.io_read_16(addr & !1);
                if addr & 1 == 0 {
                    value as u8
                } else {
                    (value >> 8) as u8
                }
            }
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
            REG_SOUND_START..=REG_SOUND_END => self.apu_io_write(addr, value),
            _ => {
                // XXX: this isn't really correct -- you can't just do a read
                // of the other 8 bits and smash it together, since not every
                // register is readable, and even some that are readable aren't
                // completely readable (e.g. sound registers are only part
                // readable).
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

/// The WAITCNT register.
#[derive(Copy, Clone)]
pub struct WaitControl(pub u16);

impl WaitControl {
    pub fn sram(self) -> u16 {
        self.0.bit_range(0..2)
    }

    pub fn ws0_nonsequential(self) -> u16 {
        self.0.bit_range(2..4)
    }

    pub fn ws0_sequential(self) -> u16 {
        self.0.bit_range(4..5)
    }

    pub fn ws1_nonsequential(self) -> u16 {
        self.0.bit_range(5..7)
    }

    pub fn ws1_sequential(self) -> u16 {
        self.0.bit_range(7..8)
    }

    pub fn ws2_nonsequential(self) -> u16 {
        self.0.bit_range(8..10)
    }

    pub fn ws2_sequential(self) -> u16 {
        self.0.bit_range(10..11)
    }

    #[allow(unused)]
    pub fn phi_terminal_output(self) -> u16 {
        self.0.bit_range(11..13)
    }

    #[allow(unused)]
    pub fn prefetch(self) -> bool {
        self.0.bit(14)
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

pub const REG_WIN0H: u32 = 0x0400_0040;
pub const REG_WIN1H: u32 = 0x0400_0042;
pub const REG_WIN0V: u32 = 0x0400_0044;
pub const REG_WIN1V: u32 = 0x0400_0046;
pub const REG_WININ: u32 = 0x0400_0048;
pub const REG_WINOUT: u32 = 0x0400_004A;
pub const REG_MOSAIC: u32 = 0x0400_004C;
pub const REG_BLDCNT: u32 = 0x0400_0050;
pub const REG_BLDALPHA: u32 = 0x0400_0052;
pub const REG_BLDY: u32 = 0x0400_0054;

pub const REG_TM0CNT_L: u32 = 0x0400_0100;
pub const REG_TM1CNT_L: u32 = 0x0400_0104;
pub const REG_TM2CNT_L: u32 = 0x0400_0108;
pub const REG_TM3CNT_L: u32 = 0x0400_010C;
pub const REG_TM0CNT_H: u32 = 0x0400_0102;
pub const REG_TM1CNT_H: u32 = 0x0400_0106;
pub const REG_TM2CNT_H: u32 = 0x0400_010A;
pub const REG_TM3CNT_H: u32 = 0x0400_010E;

pub const REG_IME: u32 = 0x0400_0208;
pub const REG_IE: u32 = 0x0400_0200;
pub const REG_IF: u32 = 0x0400_0202;
pub const REG_WAITCNT: u32 = 0x0400_0204;
pub const REG_HALTCNT: u32 = 0x0400_0301;

pub const REG_DMA_START: u32 = 0x0400_00B0;
pub const REG_DMA_END: u32 = 0x0400_00DE;

pub const REG_SOUND1CNT_L_L: u32 = 0x0400_0060;
pub const REG_SOUND1CNT_H_L: u32 = 0x0400_0062;
pub const REG_SOUND1CNT_H_H: u32 = 0x0400_0063;
pub const REG_SOUND1CNT_X_L: u32 = 0x0400_0064;
pub const REG_SOUND1CNT_X_H: u32 = 0x0400_0065;
pub const REG_SOUND2CNT_L_L: u32 = 0x0400_0068;
pub const REG_SOUND2CNT_L_H: u32 = 0x0400_0069;
pub const REG_SOUND2CNT_H_L: u32 = 0x0400_006C;
pub const REG_SOUND2CNT_H_H: u32 = 0x0400_006D;
pub const REG_SOUND3CNT_START: u32 = 0x0400_0070;
pub const REG_SOUND3CNT_END: u32 = 0x0400_0075;
pub const REG_SOUND4CNT_START: u32 = 0x0400_0078;
pub const REG_SOUND4CNT_END: u32 = 0x0400_007D;
pub const REG_SOUNDCNT_L_L: u32 = 0x0400_0080;
pub const REG_SOUNDCNT_L_H: u32 = 0x0400_0081;
pub const REG_SOUNDCNT_H_L: u32 = 0x0400_0082;
pub const REG_SOUNDCNT_H_H: u32 = 0x0400_0083;
pub const REG_SOUNDCNT_X_L: u32 = 0x0400_0084;
pub const REG_SOUNDBIAS_L: u32 = 0x0400_0088;
pub const REG_SOUNDBIAS_H: u32 = 0x0400_0089;
pub const REG_WAVE_RAM_START: u32 = 0x0400_0090;
pub const REG_WAVE_RAM_END: u32 = 0x0400_009F;
pub const REG_FIFO_A: u32 = 0x0400_00A0;
pub const REG_FIFO_B: u32 = 0x0400_00A4;
pub const REG_SOUND_START: u32 = REG_SOUND1CNT_L_L;
pub const REG_SOUND_END: u32 = 0x0400_00A8;
