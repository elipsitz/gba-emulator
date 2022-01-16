use bit::BitIndex;

use crate::io::*;
use crate::Gba;

use super::{channel::ToneRegister, CHANNEL_LEFT, CHANNEL_RIGHT};

impl Gba {
    pub(crate) fn apu_io_write(&mut self, addr: u32, value: u16) {
        match addr {
            REG_SOUND1CNT_L => self.apu.tone1.write_register(ToneRegister::Sweep, value),
            REG_SOUND1CNT_H => self.apu.tone1.write_register(ToneRegister::Duty, value),
            REG_SOUND1CNT_X => self.apu.tone1.write_register(ToneRegister::Freq, value),
            REG_SOUND2CNT_L => self.apu.tone2.write_register(ToneRegister::Duty, value),
            REG_SOUND2CNT_H => self.apu.tone2.write_register(ToneRegister::Freq, value),
            REG_SOUNDCNT_L => {
                self.apu.psg_channel_volume[1] = value.bit_range(0..3);
                self.apu.psg_channel_volume[0] = value.bit_range(4..7);
                for i in 0..4 {
                    self.apu.psg_channel_enable[1][i] = value.bit(8 + i);
                    self.apu.psg_channel_enable[0][i] = value.bit(12 + i);
                }
            }
            REG_SOUNDCNT_H => {
                self.apu.psg_mixer_volume = value.bit_range(0..2);
                for i in 0..2 {
                    self.apu.dma[i].volume = value.bit(2 + i) as u8;
                    self.apu.dma[i].channel[CHANNEL_RIGHT] = value.bit(8 + (i * 4));
                    self.apu.dma[i].channel[CHANNEL_LEFT] = value.bit(9 + (i * 4));
                    self.apu.dma[i].timer = value.bit(10 + (i * 4)) as u8;

                    let reset_fifo = value.bit(11 + (i * 4));
                    if reset_fifo {
                        self.apu.dma[i].fifo.reset();
                    }
                }
                self.timer_update();
            }
            REG_SOUNDCNT_X => {
                self.apu.master_enable = value.bit(7);
                self.timer_update();
                // TODO zero psg registers 4000060h..4000081h when disabled.
            }
            REG_SOUNDBIAS => {
                self.apu.bias_level = value.bit_range(0..10);
                self.apu.resolution = value.bit_range(14..16);
            }
            _ => {}
        }
    }

    pub(crate) fn apu_io_read(&mut self, addr: u32) -> u16 {
        match addr {
            REG_SOUND1CNT_L => self.apu.tone1.read_register(ToneRegister::Sweep),
            REG_SOUND1CNT_H => self.apu.tone1.read_register(ToneRegister::Duty),
            REG_SOUND1CNT_X => self.apu.tone1.read_register(ToneRegister::Freq),
            REG_SOUND2CNT_L => self.apu.tone2.read_register(ToneRegister::Duty),
            REG_SOUND2CNT_H => self.apu.tone2.read_register(ToneRegister::Freq),
            REG_SOUNDCNT_L => {
                (self.apu.psg_channel_volume[1] << 0)
                    | (self.apu.psg_channel_volume[0] << 4)
                    | ((self.apu.psg_channel_enable[1][0] as u16) << 8)
                    | ((self.apu.psg_channel_enable[1][1] as u16) << 9)
                    | ((self.apu.psg_channel_enable[1][2] as u16) << 10)
                    | ((self.apu.psg_channel_enable[1][3] as u16) << 11)
                    | ((self.apu.psg_channel_enable[0][0] as u16) << 12)
                    | ((self.apu.psg_channel_enable[0][1] as u16) << 13)
                    | ((self.apu.psg_channel_enable[0][2] as u16) << 14)
                    | ((self.apu.psg_channel_enable[0][3] as u16) << 15)
            }
            REG_SOUNDCNT_H => {
                (self.apu.psg_mixer_volume << 0)
                    | ((self.apu.dma[0].volume as u16) << 2)
                    | ((self.apu.dma[1].volume as u16) << 3)
                    | ((self.apu.dma[0].channel[CHANNEL_RIGHT] as u16) << 8)
                    | ((self.apu.dma[0].channel[CHANNEL_LEFT] as u16) << 9)
                    | ((self.apu.dma[0].timer as u16) << 10)
                    | ((self.apu.dma[1].channel[CHANNEL_RIGHT] as u16) << 12)
                    | ((self.apu.dma[1].channel[CHANNEL_LEFT] as u16) << 13)
                    | ((self.apu.dma[1].timer as u16) << 14)
            }
            REG_SOUNDCNT_X => {
                // TODO handle Sound 1-4 ON flags
                ((self.apu.tone1.sequencer.enabled as u16) << 0)
                    | ((self.apu.tone2.sequencer.enabled as u16) << 1)
                    | ((self.apu.master_enable as u16) << 7)
            }
            REG_SOUNDBIAS => (self.apu.bias_level) | (self.apu.resolution << 14),
            _ => 0,
        }
    }

    pub(crate) fn apu_io_fifo_write(&mut self, index: usize, value: u32) {
        let fifo = &mut self.apu.dma[index].fifo;
        for byte in value.to_le_bytes() {
            fifo.enqueue(byte as i8);
        }
    }
}
