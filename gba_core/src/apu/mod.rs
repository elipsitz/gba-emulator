mod channel;
mod registers;

use crate::{
    io::{REG_FIFO_A, REG_FIFO_B},
    scheduler::Event,
    Gba,
};
use channel::DmaChannel;
use channel::ToneChannel;

/// Audio samples per second.
pub const AUDIO_SAMPLE_RATE: usize = 32768;

/// Cycles per audio sample.
pub const CYCLES_PER_SAMPLE: usize = 512;

const CHANNEL_LEFT: usize = 0;
const CHANNEL_RIGHT: usize = 1;

/// Audio processing unit state.
pub struct Apu {
    /// Audio buffer: interleaving left/right samples.
    buffer: Vec<i16>,
    /// Current sample index.
    sample: usize,

    /// PSG Channel 1 - Tone & Sweep
    tone1: ToneChannel,
    /// PSG Channel 2 - Tone
    tone2: ToneChannel,
    /// DMA audio channels
    dma: [DmaChannel; 2],

    /// Sound 1-4 Master Channel Volume (LEFT, RIGHT) (0-7)
    psg_channel_volume: [u8; 2],
    /// Sound 1-4 Channel Enable Flags (LEFT, RIGHT))
    psg_channel_enable: [[bool; 4]; 2],
    /// Sound 1-4 Mixer Volume (50% or 100%)
    psg_mixer_volume: u8,
    /// PSG/FIFO Master Enable
    master_enable: bool,

    /// Bias level.
    bias_level: u16,
    /// Amplitude Resolution / Sampling Cycle
    resolution: u8,
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            buffer: Vec::new(),
            sample: 0,

            tone1: ToneChannel::new(true),
            tone2: ToneChannel::new(false),
            dma: [DmaChannel::new(), DmaChannel::new()],

            psg_channel_volume: [0; 2],
            psg_channel_enable: [[false; 4]; 2],
            psg_mixer_volume: 0,
            master_enable: false,
            bias_level: 0x200,
            resolution: 0,
        }
    }
}

impl Gba {
    pub(crate) fn apu_init(&mut self) {
        self.scheduler.push_event(Event::AudioSample, 0);
        self.scheduler.push_event(Event::AudioSequencerTick, 0);
    }

    pub(crate) fn apu_on_sample_event(&mut self, lateness: usize) {
        let samples = 1 + (lateness / CYCLES_PER_SAMPLE);
        let next_sample = CYCLES_PER_SAMPLE - (lateness % CYCLES_PER_SAMPLE);
        self.scheduler.push_event(Event::AudioSample, next_sample);

        for _ in 0..samples {
            let (left, right) = self.emit_sample();
            self.apu.buffer.push(left);
            self.apu.buffer.push(right);
        }
    }

    pub(crate) fn apu_on_sequencer_event(&mut self, lateness: usize) {
        const CYCLES_PER_TICK: usize = channel::Sequencer::CYCLES_PER_TICK;
        let ticks = 1 + (lateness / CYCLES_PER_TICK);
        let next_tick = CYCLES_PER_TICK - (lateness % CYCLES_PER_TICK);
        self.scheduler
            .push_event(Event::AudioSequencerTick, next_tick);

        for _ in 0..ticks {
            self.apu.tone1.sequencer.tick();
            self.apu.tone2.sequencer.tick();
            // TODO tick channels 3 and 4 as well.
        }
    }

    /// Called when a timer overflows.
    pub(crate) fn apu_on_timer_overflow(&mut self, timer: usize) {
        if self.apu.master_enable {
            for i in 0..2 {
                let channel = &mut self.apu.dma[i];
                if channel.timer == timer as u8 {
                    channel.sample = channel.fifo.dequeue();
                    if channel.fifo.len() <= 16 {
                        self.dma_notify_audio_fifo([REG_FIFO_A, REG_FIFO_B][i]);
                    }
                }
            }
        }
    }

    /// Returns whether the APU "cares about" a given timer
    /// and should thus receive events when it overflows.
    pub(crate) fn apu_needs_timer(&self, index: usize) -> bool {
        let enable_0 = self.apu.dma[0].channel[0] || self.apu.dma[0].channel[1];
        let enable_1 = self.apu.dma[1].channel[0] || self.apu.dma[1].channel[1];
        let timer_0 = enable_0 && (self.apu.dma[0].timer == index as u8);
        let timer_1 = enable_1 && (self.apu.dma[1].timer == index as u8);
        self.apu.master_enable && (timer_0 || timer_1)
    }

    /// Clear the APU buffer (at the beginning of a frame).
    pub(crate) fn apu_buffer_clear(&mut self) {
        self.apu.buffer.clear();
    }

    /// Get the current APU buffer.
    pub(crate) fn apu_buffer(&self) -> &[i16] {
        &self.apu.buffer
    }

    /// Emit a sample (left and right channels).
    fn emit_sample(&mut self) -> (i16, i16) {
        let time = self.apu.sample * CYCLES_PER_SAMPLE;
        self.apu.sample += 1;

        // TODO sample at the configured rate and then resample to the emulator output rate.
        // TODO handle master enable being off.

        // 4x the PSG mixer volume.
        let psg_volume = [1, 2, 4, 0][self.apu.psg_mixer_volume as usize];

        let mut sample = [0i16; 2];
        for channel in 0..2 {
            let mut psg = 0i16;
            if self.apu.psg_channel_enable[channel][0] {
                psg += self.apu.tone1.sample(time);
            }
            if self.apu.psg_channel_enable[channel][1] {
                psg += self.apu.tone2.sample(time);
            }
            let psg_channel_volume = self.apu.psg_channel_volume[channel] as i16;
            // Divide by 28 -- 4 for mixer volume, 7 for channel volume.
            psg = (psg * psg_volume * psg_channel_volume) / 28;
            sample[channel] += psg;

            for fifo in 0..2 {
                if self.apu.dma[fifo].channel[channel] {
                    let v = 2 << self.apu.dma[fifo].volume;
                    let s = (self.apu.dma[fifo].sample as i16) * v;
                    sample[channel] += s;
                }
            }
        }

        // Handle bias.
        for i in 0..2 {
            // Sample range is +/- 0x600.
            let input = sample[i];
            // Add bias and clamp to 0..0x3FF.
            let biased = input + (self.apu.bias_level as i16);
            let output = biased.max(0).min(0x3FF);
            // XXX: maybe just output as a float? Rescale [0, 0x400) to [-1.0, 1.0)?
            sample[i] = (output - 0x200) * 64;
        }

        (sample[0], sample[1])
    }
}
