use bit::BitIndex;

use crate::io::*;
use crate::Gba;

impl Gba {
    pub(crate) fn apu_io_write(&mut self, addr: u32, value: u16) {
        match addr {
            REG_SOUNDCNT_L => {
                self.apu.psg_volume_right = value.bit_range(0..3);
                self.apu.psg_volume_left = value.bit_range(4..7);
                for i in 0..4 {
                    self.apu.psg_enable_right[i] = value.bit(8 + i);
                    self.apu.psg_enable_left[i] = value.bit(12 + i);
                }
            }
            REG_SOUNDCNT_H => {
                self.apu.psg_volume = value.bit_range(0..2);
                for i in 0..2 {
                    self.apu.dma_volume[i] = value.bit(2 + i) as u16;
                    self.apu.dma_enable_right[i] = value.bit(8 + (i * 4));
                    self.apu.dma_enable_left[i] = value.bit(9 + (i * 4));
                    self.apu.dma_timer_select[i] = value.bit(10 + (i * 4)) as u16;
                    // TODO handle reset FIFO
                    let _reset_fifo = value.bit(11 + (i * 4));
                }
            }
            REG_SOUNDCNT_X => {
                self.apu.master_enable = value.bit(7);
                // TODO zero psg registers 4000060h..4000081h when disabled.
            }
            REG_SOUNDBIAS => {
                self.apu.bias_level = value.bit_range(1..10);
                self.apu.resolution = value.bit_range(14..16);
            }
            _ => {}
        }
    }

    pub(crate) fn apu_io_read(&mut self, addr: u32) -> u16 {
        match addr {
            REG_SOUNDCNT_L => {
                (self.apu.psg_volume_right << 0)
                    | (self.apu.psg_volume_left << 4)
                    | ((self.apu.psg_enable_right[0] as u16) << 8)
                    | ((self.apu.psg_enable_right[1] as u16) << 9)
                    | ((self.apu.psg_enable_right[2] as u16) << 10)
                    | ((self.apu.psg_enable_right[3] as u16) << 11)
                    | ((self.apu.psg_enable_left[0] as u16) << 12)
                    | ((self.apu.psg_enable_left[1] as u16) << 13)
                    | ((self.apu.psg_enable_left[2] as u16) << 14)
                    | ((self.apu.psg_enable_left[3] as u16) << 15)
            }
            REG_SOUNDCNT_H => {
                (self.apu.psg_volume << 0)
                    | ((self.apu.dma_volume[0] as u16) << 2)
                    | ((self.apu.dma_volume[1] as u16) << 3)
                    | ((self.apu.dma_enable_right[0] as u16) << 8)
                    | ((self.apu.dma_enable_left[0] as u16) << 9)
                    | ((self.apu.dma_timer_select[0]) << 10)
                    | ((self.apu.dma_enable_right[1] as u16) << 12)
                    | ((self.apu.dma_enable_left[1] as u16) << 13)
                    | ((self.apu.dma_timer_select[1]) << 14)
            }
            REG_SOUNDCNT_X => {
                // TODO handle Sound 1-4 ON flags
                (self.apu.master_enable as u16) << 7
            }
            REG_SOUNDBIAS => (self.apu.bias_level << 1) | (self.apu.resolution << 14),
            _ => 0,
        }
    }
}
