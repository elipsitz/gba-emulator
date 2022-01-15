mod dma;
mod tone;

pub use dma::DmaChannel;
pub use tone::{ToneChannel, ToneRegister};

/// Common controller for things that several channels use,
/// length, volume envelope, and sweep.
pub struct Sequencer {}

impl Sequencer {
    /// Cycles per sequencer tick -- 512 Hz.
    pub const CYCLES_PER_TICK: usize = 16777216 / 512;

    pub fn new() -> Sequencer {
        Sequencer {}
    }

    /// Frame sequencer tick. Should be called at 512Hz.
    pub fn tick(&mut self) {
        // TODO
    }
}
