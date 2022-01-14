use crate::{scheduler::Event, Gba};

mod registers;

/// Audio samples per second.
pub const AUDIO_SAMPLE_RATE: usize = 32768;

/// Cycles per audio sample.
pub const CYCLES_PER_SAMPLE: usize = 512;

/// Audio processing unit state.
pub struct Apu {
    /// Audio buffer: interleaving left/right samples.
    buffer: Vec<i16>,
    /// Current sample index.
    sample: usize,

    /// Sound 1-4 Master Volume RIGHT (0-7)
    psg_volume_right: u16,
    /// Sound 1-4 Master Volume LEFT (0-7)
    psg_volume_left: u16,
    /// Sound 1-4 Enable Flags RIGHT
    psg_enable_right: [bool; 4],
    /// Sound 1-4 Enable Flags LEFT
    psg_enable_left: [bool; 4],

    /// Sound 1-4 Volume
    psg_volume: u16,
    /// DMA Sound A/B Volume
    dma_volume: [u16; 2],
    /// DMA Sound A/B Enable Right
    dma_enable_right: [bool; 2],
    /// DMA Sound A/B Enable Left
    dma_enable_left: [bool; 2],
    /// DMA Sound A/B Timer Select (Timer 0 or Timer 1)
    dma_timer_select: [u16; 2],

    /// PSG/FIFO Master Enable
    master_enable: bool,

    /// Bias level.
    bias_level: u16,
    /// Amplitude Resolution / Sampling Cycle
    resolution: u16,
}

impl Apu {
    pub fn new() -> Apu {
        Apu {
            buffer: Vec::new(),
            sample: 0,

            psg_volume_left: 0,
            psg_volume_right: 0,
            psg_enable_left: [false; 4],
            psg_enable_right: [false; 4],
            psg_volume: 0,
            dma_volume: [0; 2],
            dma_enable_left: [false; 2],
            dma_enable_right: [false; 2],
            dma_timer_select: [0; 2],
            master_enable: false,
            bias_level: 0x100,
            resolution: 0,
        }
    }
}

impl Gba {
    pub(crate) fn apu_init(&mut self) {
        self.scheduler
            .push_event(Event::AudioSample, CYCLES_PER_SAMPLE);
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
        self.apu.sample += 1;

        // 440Hz is about 1 cycle per 74 samples.
        let val = (((self.apu.sample as f64) / (74.0)) * 6.28).sin() * 16_000.0;

        // Shift back and forth between left and right.
        let chanshift = ((self.apu.sample as f64) / (512.0 * 60.0 * 2.0) * 6.28).sin();
        let left = val * ((1.0 - chanshift) / 2.0);
        let right = val * ((chanshift + 1.0) / 2.0);

        (left.round() as i16, right.round() as i16)
    }
}
