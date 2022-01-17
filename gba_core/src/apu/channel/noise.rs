use bit::BitIndex;

use super::{EnvelopeDirection, Sequencer};

const LEN_7: usize = 0x7F;
const LEN_15: usize = 0x7FFF;

#[derive(Copy, Clone)]
enum LfsrWidth {
    Width15 = 0,
    Width7 = 1,
}

pub struct NoiseChannel {
    pub sequencer: Sequencer,

    /// Dividing ratio of frequencies.
    freq_r: u8,
    /// Shift clock frequency.
    freq_s: u8,
    /// LSFR width (0 = 15 bits, 1 = 7 bits)
    width: LfsrWidth,
}

impl NoiseChannel {
    pub fn new() -> NoiseChannel {
        NoiseChannel {
            sequencer: Sequencer::new(64),
            freq_r: 0,
            freq_s: 0,
            width: LfsrWidth::Width15,
        }
    }

    pub fn enabled(&self) -> bool {
        self.sequencer.enabled
    }

    pub fn sample(&self, time: usize) -> i16 {
        if !self.enabled() {
            return 0;
        }

        // freq = 524288 Hz / r / 2^(s+1)
        // period in cycles = (r * 2^s) * 64
        // r = 0 actually means r = 0.5.
        let period = if self.freq_r == 0 {
            32 << (self.freq_s as usize)
        } else {
            (64 << (self.freq_s as usize)) * (self.freq_r as usize)
        };

        let state = match self.width {
            LfsrWidth::Width15 => TABLE_15[(time / period) % LEN_15],
            LfsrWidth::Width7 => TABLE_7[(time / period) % LEN_7] as u16,
        };
        let sample = if state & 1 == 1 { 8 } else { -8 };
        let volume = self.sequencer.envelope_volume as i16;

        sample * volume
    }

    pub fn read_register(&mut self, register: u32) -> u8 {
        match register {
            0 => 0,
            1 => {
                (self.sequencer.envelope_time << 0)
                    | ((self.sequencer.envelope_direction as u8) << 3)
                    | (self.sequencer.envelope_initial << 4)
            }
            4 => (self.freq_r << 0) | ((self.width as u8) << 3) | (self.freq_s << 4),
            5 => ((self.sequencer.length_enabled) as u8) << 6,
            _ => 0,
        }
    }

    pub fn write_register(&mut self, register: u32, value: u8) {
        match register {
            0 => {
                self.sequencer.length_counter = 64 - (value.bit_range(0..6) as u16);
            }
            1 => {
                self.sequencer.envelope_time = value.bit_range(0..3);
                self.sequencer.envelope_direction = if value.bit(3) {
                    EnvelopeDirection::Increase
                } else {
                    EnvelopeDirection::Decrease
                };
                self.sequencer.envelope_initial = value.bit_range(4..8);
            }
            4 => {
                self.freq_r = value.bit_range(0..3);
                self.width = if value.bit(3) {
                    LfsrWidth::Width7
                } else {
                    LfsrWidth::Width15
                };
                self.freq_s = value.bit_range(4..8);
            }
            5 => {
                self.sequencer.length_enabled = value.bit(6);

                if value.bit(7) {
                    // XXX: reset sequence too: "Noise channel's LFSR bits are all set to 1."
                    self.sequencer.restart();
                }
            }
            _ => {}
        }
    }
}

const fn make_table_7() -> [u8; LEN_7] {
    let mut table = [0; LEN_7];
    let mut lfsr = 0x40;
    let mut i = 0;
    while i < LEN_7 {
        let carry = lfsr & 1;
        lfsr >>= 1;
        if carry == 1 {
            lfsr ^= 0x60;
        }
        table[i] = lfsr;
        i += 1;
    }
    table
}

const fn make_table_15() -> [u16; LEN_15] {
    let mut table = [0; LEN_15];
    let mut lfsr = 0x4000;
    let mut i = 0;
    while i < LEN_15 {
        let carry = lfsr & 1;
        lfsr >>= 1;
        if carry == 1 {
            lfsr ^= 0x6000;
        }
        table[i] = lfsr;
        i += 1;
    }
    table
}

static TABLE_7: [u8; LEN_7] = make_table_7();
static TABLE_15: [u16; LEN_15] = make_table_15();
