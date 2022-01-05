use std::hint::unreachable_unchecked;

use crate::Gba;
use bit::BitIndex;

/// State for the DMA controller.
pub struct Dma {
    channels: [DmaChannel; 4],
}

/// A single DMA channel.
#[derive(Default)]
struct DmaChannel {
    /// Source address register.
    src: u32,
    /// Destination address register.
    dest: u32,
    /// Number of transfers (word count).
    count: u16,
    /// Control register.
    control: DmaChannelControl,

    /// Internal source address register.
    internal_src: u32,
    /// Internal destination address register.
    internal_dest: u32,
    /// Internal count register.
    internal_count: u16,
}

/// DMA control register.
#[derive(Copy, Clone, Default)]
struct DmaChannelControl(u16);

#[derive(Copy, Clone, PartialEq, Eq)]
enum AdjustmentMode {
    Increment,
    Decrement,
    Fixed,
    IncrementReload,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum TimingMode {
    Immediate,
    VBlank,
    HBlank,
    Special,
}

impl DmaChannelControl {
    fn dest_adjustment(self) -> AdjustmentMode {
        match self.0.bit_range(5..7) {
            0b00 => AdjustmentMode::Increment,
            0b01 => AdjustmentMode::Decrement,
            0b10 => AdjustmentMode::Fixed,
            0b11 => AdjustmentMode::IncrementReload,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn src_adjustment(self) -> AdjustmentMode {
        match self.0.bit_range(7..9) {
            0b00 => AdjustmentMode::Increment,
            0b01 => AdjustmentMode::Decrement,
            0b10 => AdjustmentMode::Fixed,
            0b11 => AdjustmentMode::IncrementReload,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn repeat(self) -> bool {
        self.0.bit(9)
    }

    fn word_size(self) -> usize {
        if self.0.bit(10) {
            4
        } else {
            2
        }
    }

    fn timing(self) -> TimingMode {
        match self.0.bit_range(12..14) {
            0b00 => TimingMode::Immediate,
            0b01 => TimingMode::VBlank,
            0b10 => TimingMode::HBlank,
            0b11 => TimingMode::Special,
            _ => unsafe { unreachable_unchecked() },
        }
    }

    fn irq(self) -> bool {
        self.0.bit(14)
    }

    fn enabled(self) -> bool {
        self.0.bit(15)
    }
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            channels: <[DmaChannel; 4]>::default(),
        }
    }
}

impl Gba {
    /// Handle a 16-bit write to a DMA register.
    ///
    /// `reg` is relative to the start of the DMA register region, 0x0400_00B0.
    pub(crate) fn dma_reg_write(&mut self, reg: u32, value: u16) {
        let channel_index = (reg / 12) as usize;
        let reg = reg % 12;
        let mut c = &mut self.dma.channels[channel_index];
        match reg {
            // Source Address (28 bits).
            // XXX: see if different channels have different widths.
            0x0 => (c.src = (c.src & 0xFFFF_0000) | (value as u32)),
            0x2 => (c.src = (c.src & 0x0000_FFFF) | (((value as u32) & 0x0FFF) << 16)),
            // Destination Address (27 bits).
            0x4 => (c.dest = (c.dest & 0xFFFF_0000) | (value as u32)),
            0x6 => (c.dest = (c.dest & 0x0000_FFFF) | (((value as u32) & 0x07FF) << 16)),
            // Transfer count.
            // XXX: see if different channels have different maximum counts.
            0x8 => c.count = value,
            // Control register.
            0xA => {
                let _control = DmaChannelControl(value);
                todo!();
            }
            _ => unsafe { unreachable_unchecked() },
        }
    }

    /// Handle a 16-bit read from a DMA register.
    ///
    /// `reg` is relative to the start of the DMA register region, 0x0400_00B0.
    pub(crate) fn dma_reg_read(&mut self, reg: u32) -> u16 {
        let channel_index = (reg / 12) as usize;
        let reg = reg % 12;
        if reg == 0xA {
            // DMA Control
            self.dma.channels[channel_index].control.0
        } else {
            0
        }
    }
}
