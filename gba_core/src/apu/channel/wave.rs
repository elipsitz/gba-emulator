use bit::BitIndex;
use serde::{Deserialize, Serialize};

use super::Sequencer;

#[derive(Serialize, Deserialize)]
pub struct WaveChannel {
    pub sequencer: Sequencer,

    /// 0 = One bank / 32 digits, 1 = Two banks / 64 digits
    bank_dimension: u8,
    bank_number: u8,
    playing: bool,
    /// (0=Mute/Zero, 1=100%, 2=50%, 3=25%)
    volume: u8,
    /// (0=Use above, 1=Force 75% regardless of above)
    volume_force: bool,
    /// Sample rate: 2097152/(2048-n) Hz   (0-2047)
    sample_rate: u16,

    /// Wave RAM. Two separate banks of 16 bytes each.
    wave_ram: [u8; 32],
}

impl WaveChannel {
    pub fn new() -> WaveChannel {
        WaveChannel {
            sequencer: Sequencer::new(256),
            bank_dimension: 0,
            bank_number: 0,
            playing: false,
            sample_rate: 0,
            volume: 0,
            volume_force: false,
            wave_ram: [0; 32],
        }
    }

    pub fn enabled(&self) -> bool {
        self.sequencer.enabled && self.playing
    }

    pub fn sample(&self, time: usize) -> i16 {
        if !self.enabled() {
            return 0;
        }

        // TODO: fix discontinuities when the frequency changes, sound is stopped, etc.
        // How long one sample is played (in cycles).
        let period = (2048 - (self.sample_rate as usize)) * 8;
        let sample_count = if self.bank_dimension == 0 { 32 } else { 64 };
        let sample_offset = (self.bank_number as usize) * 32;
        let sample_index = (time / period) % sample_count;
        let sample_index = (sample_index + sample_offset) % 64;

        let sample_byte = self.wave_ram[sample_index / 2];
        let sample = if sample_index % 2 == 0 {
            sample_byte >> 4
        } else {
            sample_byte & 0x0F
        };

        // Sample is 4bits, signed (so subtract 8).
        // That puts us in range (-8, 7)... we need to be roughly in (-128, 128)
        // So multiply by 16 at max volume.
        let volume = if self.volume_force {
            3 * 4
        } else {
            [0, 4, 2, 1][self.volume as usize] * 4
        };

        ((sample as i16) - 8) * volume
    }

    pub fn read_register(&mut self, register: u32) -> u8 {
        match register {
            0 => (self.bank_dimension << 5) | (self.bank_number << 6) | ((self.playing as u8) << 7),
            2 => self.sequencer.length_counter as u8,
            3 => (self.volume << 5) | ((self.volume_force as u8) << 7),
            4 => 0,
            5 => ((self.sequencer.length_enabled) as u8) << 6,
            _ => 0,
        }
    }

    pub fn write_register(&mut self, register: u32, value: u8) {
        match register {
            0 => {
                self.bank_dimension = value.bit(5) as u8;
                self.bank_number = value.bit(6) as u8;
                self.playing = value.bit(7);
            }
            2 => {
                self.sequencer.length_counter = 256 - (value as u16);
            }
            3 => {
                self.volume = value.bit_range(5..7);
                self.volume_force = value.bit(7);
            }
            4 => {
                self.sample_rate
                    .set_bit_range(0..8, value.bit_range(0..8) as u16);
            }
            5 => {
                self.sample_rate
                    .set_bit_range(8..11, value.bit_range(0..3) as u16);
                self.sequencer.length_enabled = value.bit(6);

                if value.bit(7) {
                    // XXX: should also restart sample at the beginning
                    self.sequencer.restart();
                }
            }
            _ => {}
        }
    }

    pub fn write_wave_ram(&mut self, offset: u32, value: u8) {
        // Write to the other bank.
        let bank = (1 - self.bank_number) as usize;
        let index = (bank * 16) + (offset as usize);
        self.wave_ram[index] = value;
    }

    pub fn read_wave_ram(&mut self, offset: u32) -> u8 {
        // Read from the other bank.
        let bank = (1 - self.bank_number) as usize;
        let index = (bank * 16) + (offset as usize);
        self.wave_ram[index]
    }
}
