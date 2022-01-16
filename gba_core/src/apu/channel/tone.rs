use bit::BitIndex;

use crate::apu::channel::EnvelopeDirection;

use super::{Sequencer, SweepDirection};

pub struct ToneChannel {
    has_sweep: bool,
    pub sequencer: Sequencer,

    /// Wave duty type (0-3).
    duty: u8,
}

#[derive(Debug)]
pub enum ToneRegister {
    SweepL,
    DutyL,
    DutyH,
    FreqL,
    FreqH,
}

impl ToneChannel {
    pub fn new(has_sweep: bool) -> ToneChannel {
        ToneChannel {
            has_sweep,
            sequencer: Sequencer::new(64),
            duty: 0,
        }
    }

    pub fn sample(&self, time: usize) -> i16 {
        if !self.sequencer.enabled {
            return 0;
        }

        // Time is relative to the system clock.
        const DUTY_PATTERN: [[i16; 8]; 4] = [
            [8, -8, -8, -8, -8, -8, -8, -8],
            [8, 8, -8, -8, -8, -8, -8, -8],
            [8, 8, 8, 8, -8, -8, -8, -8],
            [8, 8, 8, 8, 8, 8, -8, -8],
        ];

        // Sampling period: 128 cycles is 131072 Hz
        // TODO: fix discontinuities when the frequency changes
        // In practice, the hardware timer works by counting down to 0 from the
        // period -- our method causes the index to jump when we change the frequency
        // because we're always redividing from the system clock count.
        let freq = self.sequencer.sweep_current_freq as usize;
        let period = (2048 - freq) * 128;
        let index = ((time * 8) / period) % 8;

        let volume = self.sequencer.envelope_volume as i16;
        DUTY_PATTERN[self.duty as usize][index] * volume
    }

    pub fn read_register(&mut self, register: ToneRegister) -> u8 {
        match register {
            ToneRegister::SweepL if self.has_sweep => {
                (self.sequencer.sweep_shift << 0)
                    | ((self.sequencer.sweep_direction as u8) << 3)
                    | (self.sequencer.sweep_time << 4)
            }
            ToneRegister::SweepL => 0,
            ToneRegister::DutyL => self.duty << 6,
            ToneRegister::DutyH => {
                (self.sequencer.envelope_time << 0)
                    | ((self.sequencer.envelope_direction as u8) << 3)
                    | (self.sequencer.envelope_initial << 4)
            }
            ToneRegister::FreqL => 0,
            ToneRegister::FreqH => ((self.sequencer.length_enabled) as u8) << 6,
        }
    }

    pub fn write_register(&mut self, register: ToneRegister, value: u8) {
        match register {
            ToneRegister::SweepL if self.has_sweep => {
                self.sequencer.sweep_shift = value.bit_range(0..3);
                self.sequencer.sweep_direction = if value.bit(3) {
                    SweepDirection::Decrease
                } else {
                    SweepDirection::Increase
                };
                self.sequencer.sweep_time = value.bit_range(4..7);
            }
            ToneRegister::SweepL => {}
            ToneRegister::DutyL => {
                self.sequencer.length_counter = 64 - (value.bit_range(0..6) as u16);
                self.duty = value.bit_range(6..8);
            }
            ToneRegister::DutyH => {
                self.sequencer.envelope_time = value.bit_range(0..3);
                self.sequencer.envelope_direction = if value.bit(3) {
                    EnvelopeDirection::Increase
                } else {
                    EnvelopeDirection::Decrease
                };
                self.sequencer.envelope_initial = value.bit_range(4..8);
            }
            ToneRegister::FreqL => {
                self.sequencer
                    .sweep_initial_freq
                    .set_bit_range(0..8, value.bit_range(0..8) as u16);
                self.sequencer.sweep_current_freq = self.sequencer.sweep_initial_freq;
            }
            ToneRegister::FreqH => {
                self.sequencer
                    .sweep_initial_freq
                    .set_bit_range(8..11, value.bit_range(0..3) as u16);
                self.sequencer.sweep_current_freq = self.sequencer.sweep_initial_freq;
                self.sequencer.length_enabled = value.bit(6);

                if value.bit(7) {
                    self.sequencer.restart();
                }
            }
        }
    }
}
