use crate::Gba;
use bit::BitIndex;

/// State for the interrupt manager.
pub struct InterruptManager {
    /// Whether interrupts are enabled.
    pub global_enabled: bool,

    /// Individual interrupts that are enabled.
    pub enabled: u16,

    /// Individual interrupts that are pending.
    pub pending: u16,
}

#[derive(Copy, Clone)]
#[allow(unused)]
pub enum InterruptKind {
    VBlank = 0,
    HBlank = 1,
    VCount = 2,
    Timer0 = 3,
    Timer1 = 4,
    Timer2 = 5,
    Timer3 = 6,
    Serial = 7,
    Dma0 = 8,
    Dma1 = 9,
    Dma2 = 10,
    Dma3 = 11,
    Keypad = 12,
    Gamepak = 13,
}

impl InterruptManager {
    pub fn new() -> InterruptManager {
        InterruptManager {
            global_enabled: false,
            enabled: 0,
            pending: 0,
        }
    }
}

impl Gba {
    /// Handle a write to the "Interrupt Request / Acknowledge" register.
    /// Acknowledges some IRQs.
    #[inline(always)]
    pub(crate) fn interrupt_reg_if_write(&mut self, acked: u16) {
        // For each 1 in "acked", set corresponding pending to false.
        // This is the "not implies" boolean operation, (a & ~b)
        self.interrupt.pending &= !acked;
    }

    /// Checks whether an interrupt is pending (IRQ).
    #[inline(always)]
    pub(crate) fn interrupt_pending(&self) -> bool {
        self.interrupt.global_enabled && ((self.interrupt.pending & self.interrupt.enabled) != 0)
    }

    /// Raise an interrupt.
    pub(crate) fn interrupt_raise(&mut self, kind: InterruptKind) {
        self.interrupt.pending.set_bit(kind as usize, true);
    }
}
