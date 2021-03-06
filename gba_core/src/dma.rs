use std::hint::unreachable_unchecked;

use crate::{
    bus::MemoryAccessType,
    interrupt::InterruptKind,
    io::{REG_FIFO_A, REG_FIFO_B},
    Gba,
};
use bit::BitIndex;
use serde::{Deserialize, Serialize};

const NUM_CHANNELS: usize = 4;

/// State for the DMA controller.
#[derive(Serialize, Deserialize)]
pub struct Dma {
    channels: [DmaChannel; NUM_CHANNELS],
    /// Active channel bitfield.
    active: u8,
}

/// A single DMA channel.
#[derive(Serialize, Deserialize)]
struct DmaChannel {
    /// Source address register.
    src: u32,
    /// Destination address register.
    dest: u32,
    /// Number of transfers (word count).
    count: u16,
    /// Control register.
    control: DmaChannelControl,

    /// Next access type.
    access_type: MemoryAccessType,
    /// Whether this is set up for FIFO.
    fifo: bool,

    /// Internal source address register.
    internal_src: u32,
    /// Internal destination address register.
    internal_dest: u32,
    /// Internal count register.
    internal_count: u32,
}

impl Default for DmaChannel {
    fn default() -> Self {
        DmaChannel {
            src: 0,
            dest: 0,
            count: 0,
            control: DmaChannelControl(0),
            access_type: MemoryAccessType::NonSequential,
            fifo: false,
            internal_src: 0,
            internal_dest: 0,
            internal_count: 0,
        }
    }
}

/// DMA control register.
#[derive(Copy, Clone, Serialize, Deserialize)]
struct DmaChannelControl(u16);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum AdjustmentMode {
    Increment,
    Decrement,
    Fixed,
    IncrementReload,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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

    fn set_enabled(&mut self, enabled: bool) {
        self.0.set_bit(15, enabled);
    }
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            active: 0,
            channels: <[DmaChannel; 4]>::default(),
        }
    }

    /// Return the transfer size of the DMA channel, if it's active.
    pub fn transfer_size(&self, channel: usize) -> Option<u16> {
        if self.active.bit(channel) {
            Some(self.channels[channel].count)
        } else {
            None
        }
    }
}

impl Gba {
    /// Returns whether any DMA channel is *active*.
    /// This is different from *enabled*: active means it's taking control and transferring.
    pub(crate) fn dma_active(&self) -> bool {
        self.dma.active != 0
    }

    /// Performs the actual DMA transfer.
    pub(crate) fn dma_step(&mut self) {
        // XXX: determine whether we need to go one cycle at a time
        // (e.g. for interaction with interrupts, DMAs of different priorities)

        for channel in 0..NUM_CHANNELS {
            // From high to low priority.
            if self.dma.active.bit(channel) {
                self.transfer_channel(channel);
            }
        }
    }

    /// Perform a DMA transfer for the given channel.
    fn transfer_channel(&mut self, index: usize) {
        // Do a single transfer.
        let channel = &self.dma.channels[index];
        let access = channel.access_type;
        let src = channel.internal_src;
        let dest = channel.internal_dest;

        let word_size;
        let src_adjust;
        let dest_adjust;
        if channel.fifo {
            word_size = 4;
            src_adjust = channel.control.src_adjustment();
            dest_adjust = AdjustmentMode::Fixed;
        } else {
            src_adjust = channel.control.src_adjustment();
            word_size = channel.control.word_size() as u32;
            dest_adjust = channel.control.dest_adjustment();
        }

        if word_size == 2 {
            let data = self.cpu_load16(src & !0b1, access);
            self.cpu_store16(dest & !0b1, data, access);
        } else {
            let data = self.cpu_load32(src & !0b11, access);
            self.cpu_store32(dest & !0b11, data, access);
        }

        // Update the state.
        let mut channel = &mut self.dma.channels[index];
        channel.access_type = MemoryAccessType::Sequential;
        match src_adjust {
            AdjustmentMode::Fixed => {}
            AdjustmentMode::Decrement => channel.internal_src = src.wrapping_sub(word_size),
            AdjustmentMode::Increment => channel.internal_src = src.wrapping_add(word_size),
            _ => unreachable!(),
        };
        match dest_adjust {
            AdjustmentMode::Fixed => {}
            AdjustmentMode::Decrement => channel.internal_dest = dest.wrapping_sub(word_size),
            AdjustmentMode::Increment | AdjustmentMode::IncrementReload => {
                channel.internal_dest = dest.wrapping_add(word_size)
            }
        };

        channel.internal_count -= 1;
        if channel.internal_count == 0 {
            // We completed the DMA.
            if channel.control.repeat() {
                if channel.fifo {
                    channel.internal_count = 4;
                } else {
                    if channel.control.dest_adjustment() == AdjustmentMode::IncrementReload {
                        channel.internal_dest = channel.dest;
                    }
                    channel.internal_count = if channel.count == 0 {
                        if index == 3 {
                            0x10000
                        } else {
                            0x4000
                        }
                    } else {
                        channel.count as u32
                    };
                }
            } else {
                channel.control.set_enabled(false);
            }

            if channel.control.irq() {
                let interrupt_kind = match index {
                    0 => InterruptKind::Dma0,
                    1 => InterruptKind::Dma1,
                    2 => InterruptKind::Dma2,
                    3 => InterruptKind::Dma3,
                    _ => unsafe { unreachable_unchecked() },
                };
                self.interrupt_raise(interrupt_kind);
            }

            self.dma.active.set_bit(index, false);
        }
    }

    /// Activate a DMA channel (in response to an event).
    pub(crate) fn dma_activate_channel(&mut self, channel: usize) {
        self.dma.active.set_bit(channel, true);
        self.dma.channels[channel].access_type = MemoryAccessType::NonSequential;
    }

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
            0x6 => (c.dest = (c.dest & 0x0000_FFFF) | (((value as u32) & 0x0FFF) << 16)),
            // Transfer count.
            // XXX: see if different channels have different maximum counts.
            0x8 => c.count = value,
            // Control register.
            0xA => {
                let control = DmaChannelControl(value);
                let enabled = !c.control.enabled() && control.enabled();

                if control.src_adjustment() == AdjustmentMode::IncrementReload {
                    panic!("Invalid DMA src adjustment IncrementReload");
                }

                if enabled {
                    // Just enabled this channel. Copy registers to internal.
                    c.internal_src = c.src;
                    c.internal_dest = c.dest;
                    c.internal_count = if c.count == 0 {
                        if channel_index == 3 {
                            0x10000
                        } else {
                            0x4000
                        }
                    } else {
                        c.count as u32
                    };

                    // Handle special audio FIFO mode.
                    c.fifo = (control.timing() == TimingMode::Special)
                        && control.repeat()
                        && (channel_index == 1 || channel_index == 2)
                        && (c.dest == REG_FIFO_A || c.dest == REG_FIFO_B);
                    if c.fifo {
                        c.internal_count = 4;
                    }

                    if control.timing() == TimingMode::Immediate {
                        let event = crate::scheduler::Event::DmaActivate(channel_index as u8);
                        self.scheduler.push_event(event, 2);
                    }
                }

                c.control = control;
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

    /// Called by the PPU on vblank.
    pub(crate) fn dma_notify_vblank(&mut self) {
        for i in 0..NUM_CHANNELS {
            let channel = &self.dma.channels[i];
            if channel.control.enabled() && channel.control.timing() == TimingMode::VBlank {
                self.dma_activate_channel(i);
            }
        }
    }

    /// Called by the PPU on hblank (only during visible, non-vblank lines).
    pub(crate) fn dma_notify_hblank(&mut self) {
        for i in 0..NUM_CHANNELS {
            let channel = &self.dma.channels[i];
            if channel.control.enabled() && channel.control.timing() == TimingMode::HBlank {
                self.dma_activate_channel(i);
            }
        }
    }

    /// Called by the PPU on "video" updates (hblank during scanlines 2..162).
    pub(crate) fn dma_notify_video(&mut self) {
        // Only relevant for channel 3.
        let i = 3;
        let channel = &self.dma.channels[i];
        if channel.control.enabled() && channel.control.timing() == TimingMode::Special {
            self.dma_activate_channel(i);
        }
    }

    /// Called by the APU when it's requesting DMA to a FIFO.
    pub(crate) fn dma_notify_audio_fifo(&mut self, addr: u32) {
        // DMA channels 1 and 2 could support audio FIFO.
        for i in 1..=2 {
            let channel = &self.dma.channels[i];
            if channel.control.enabled()
                && channel.control.timing() == TimingMode::Special
                && channel.dest == addr
            {
                self.dma_activate_channel(i);
            }
        }
    }
}
