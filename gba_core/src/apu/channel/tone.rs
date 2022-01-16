use bit::BitIndex;

use crate::apu::channel::EnvelopeDirection;

use super::{Sequencer, SweepDirection};

pub struct ToneChannel {
    has_sweep: bool,
    pub sequencer: Sequencer,

    /// Wave duty type (0-3).
    duty: u16,
}

pub enum ToneRegister {
    Sweep,
    Duty,
    Freq,
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

    pub fn read_register(&mut self, register: ToneRegister) -> u16 {
        match register {
            ToneRegister::Sweep if self.has_sweep => {
                (self.sequencer.sweep_shift << 0)
                    | ((self.sequencer.sweep_direction as u16) << 3)
                    | ((self.sequencer.sweep_time as u16) << 4)
            }
            ToneRegister::Sweep => 0,
            ToneRegister::Duty => {
                (self.duty << 6)
                    | (self.sequencer.envelope_time << 8)
                    | ((self.sequencer.envelope_direction as u16) << 11)
                    | (self.sequencer.envelope_initial << 12)
            }
            ToneRegister::Freq => ((self.sequencer.length_enabled) as u16) << 14,
        }
    }

    pub fn write_register(&mut self, register: ToneRegister, value: u16) {
        match register {
            ToneRegister::Sweep if self.has_sweep => {
                self.sequencer.sweep_shift = value.bit_range(0..3);
                self.sequencer.sweep_direction = if value.bit(3) {
                    SweepDirection::Decrease
                } else {
                    SweepDirection::Increase
                };
                self.sequencer.sweep_time = value.bit_range(4..7);
            }
            ToneRegister::Sweep => {}
            ToneRegister::Duty => {
                self.sequencer.length_counter = 64 - value.bit_range(0..6);
                self.duty = value.bit_range(6..8);
                self.sequencer.envelope_time = value.bit_range(8..11);
                self.sequencer.envelope_direction = if value.bit(11) {
                    EnvelopeDirection::Increase
                } else {
                    EnvelopeDirection::Decrease
                };
                self.sequencer.envelope_initial = value.bit_range(12..16);
            }
            ToneRegister::Freq => {
                self.sequencer.sweep_initial_freq = value.bit_range(0..11);
                self.sequencer.sweep_current_freq = self.sequencer.sweep_initial_freq;
                self.sequencer.length_enabled = value.bit(14);

                if value.bit(15) {
                    self.sequencer.restart();
                }
            }
        }
    }
}
