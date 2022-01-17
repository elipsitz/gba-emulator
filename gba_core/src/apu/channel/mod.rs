mod dma;
mod tone;
mod wave;

pub use dma::DmaChannel;
pub use tone::{ToneChannel, ToneRegister};
pub use wave::WaveChannel;

/// Common controller for things that several channels use,
/// length, volume envelope, and sweep.
pub struct Sequencer {
    step: u8,
    /// Whether the channel is enabled (may be disabled by length or sweep).
    pub enabled: bool,

    /// Change of frequency at each sweep shift.
    sweep_shift: u8,
    /// Whether the sweep causes frequency to increase or decrease.
    sweep_direction: SweepDirection,
    /// Sweep steps between frequency changes, or 0 for no sweep.
    sweep_time: u8,
    /// Current sweep step counter (counts from `time` down to 0).
    sweep_step: u8,
    /// Sweep initial frequency.
    sweep_initial_freq: u16,
    /// Sweep current frequency.
    sweep_current_freq: u16,
    /// Sweep shadow frequency.
    sweep_shadow: u16,
    /// Sweep enabled.
    sweep_enabled: bool,

    /// Default length (64 for tone and noise, 256 for wave).
    length_default: u16,
    /// Length counter.
    length_counter: u16,
    /// Whether length unit is enabled.
    length_enabled: bool,

    /// Current step counter for the envelope unit (from `time` down to 0).
    envelope_step: u8,
    /// Envelope steps between volume changes.
    envelope_time: u8,
    /// Whether the envelope causes volume to increase or decrease.
    envelope_direction: EnvelopeDirection,
    /// Initial volume of the envelope.
    envelope_initial: u8,
    /// Volume of the envelope.
    envelope_volume: u8,
    /// Envelope enabled.
    envelope_enabled: bool,
}

impl Sequencer {
    /// Cycles per sequencer tick -- 512 Hz.
    pub const CYCLES_PER_TICK: usize = 16777216 / 512;

    pub fn new(length_default: u16) -> Sequencer {
        Sequencer {
            step: 0,
            enabled: false,

            sweep_step: 0,
            sweep_shift: 0,
            sweep_direction: SweepDirection::Increase,
            sweep_time: 0,
            sweep_current_freq: 0,
            sweep_initial_freq: 0,
            sweep_shadow: 0,
            sweep_enabled: false,

            length_default,
            length_counter: 0,
            length_enabled: false,

            envelope_step: 0,
            envelope_time: 0,
            envelope_direction: EnvelopeDirection::Decrease,
            envelope_initial: 0,
            envelope_volume: 0,
            envelope_enabled: false,
        }
    }

    /// Frame sequencer tick. Should be called at 512Hz.
    pub fn tick(&mut self) {
        if self.step % 2 == 0 {
            // Update length.
            if self.enabled && self.length_enabled {
                self.length_counter -= 1;
                if self.length_counter == 0 {
                    self.enabled = false;
                }
            }
        }
        if self.step % 4 == 2 && self.sweep_enabled {
            // Update frequency sweep.
            self.sweep_step -= 1;
            if self.sweep_step == 0 {
                self.sweep_step = self.sweep_time;

                let offset = (self.sweep_shadow >> self.sweep_shift) as i16;
                let new_freq = match self.sweep_direction {
                    SweepDirection::Increase => (self.sweep_shadow as i16) + offset,
                    SweepDirection::Decrease => (self.sweep_shadow as i16) - offset,
                };
                if new_freq >= 2048 {
                    self.enabled = false;
                } else if new_freq >= 0 && self.sweep_shift != 0 {
                    self.sweep_shadow = new_freq as u16;
                    self.sweep_current_freq = new_freq as u16;
                }
            }
        }
        if self.step == 7 && self.envelope_enabled {
            self.envelope_step -= 1;
            if self.envelope_step == 0 {
                self.envelope_step = self.envelope_time;

                match self.envelope_direction {
                    EnvelopeDirection::Increase if self.envelope_volume < 15 => {
                        self.envelope_volume += 1
                    }
                    EnvelopeDirection::Decrease if self.envelope_volume > 0 => {
                        self.envelope_volume -= 1
                    }
                    _ => self.envelope_enabled = false,
                }
            }
        }

        self.step = (self.step + 1) % 8;
    }

    /// Restart the sound channel.
    pub fn restart(&mut self) {
        self.enabled = true;
        self.step = 0;

        self.sweep_enabled = (self.sweep_time != 0) || (self.sweep_time != 0);
        self.sweep_current_freq = self.sweep_initial_freq;
        self.sweep_shadow = self.sweep_initial_freq;
        self.sweep_step = self.sweep_time;

        if self.length_counter == 0 {
            self.length_counter = self.length_default;
        }

        self.envelope_volume = self.envelope_initial;
        self.envelope_step = self.envelope_time;
        self.envelope_enabled = self.envelope_time != 0;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SweepDirection {
    Increase = 0,
    Decrease = 1,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnvelopeDirection {
    Decrease = 0,
    Increase = 1,
}
