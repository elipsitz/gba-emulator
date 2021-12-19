use super::{Gba, InstructionResult};

/// A function that can execute a Thumb instruction.
type ThumbHandler = fn(&mut Gba, inst: u16) -> InstructionResult;

/// Dummy unimplemented / invalid Thumb instruction.
fn thumb_unimplemented(_s: &mut Gba, inst: u16) -> InstructionResult {
    panic!(
        "Unknown Thumb instruction: {:04x} / [{:04b} {:04b} {:02b}] {:02b} {:04b}",
        inst,
        (inst >> 12) & 0b1111,
        (inst >> 8) & 0b1111,
        (inst >> 6) & 0b11,
        (inst >> 4) & 0b11,
        (inst >> 0) & 0b1111,
    );
}

// Include look-up table for instruction handlers.
include!(concat!(env!("OUT_DIR"), "/thumb_table.rs"));

impl Gba {
    /// Get the program counter of the *currently executing Thumb instruction*.
    pub fn cpu_thumb_pc(&self) -> u32 {
        // Go back 2 instructions (because pipelining).
        self.cpu.pc.wrapping_sub(4)
    }

    /// Execute the given Thumb instruction.
    pub(super) fn cpu_execute_thumb(&mut self, inst: u16) -> InstructionResult {
        let key = ((inst >> 6) & 0x3ff) as usize;
        (THUMB_HANDLERS[key])(self, inst)
    }
}
