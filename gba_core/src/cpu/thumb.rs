use super::{alu, Gba, InstructionResult};
use bit::BitIndex;

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

/// THUMB.3: move/compare/add/subtract immediate.
fn thumb_exec_alu_immediate<const OPCODE: u16, const REG_D: u16>(
    s: &mut Gba,
    inst: u16,
) -> InstructionResult {
    let reg_d = REG_D as usize;
    let op1 = s.cpu_reg_get(reg_d);
    let op2 = inst.bit_range(0..8) as u32;

    // Decode operation.
    use alu::AluOpcode::*;
    let opcode = match OPCODE {
        0b00 => MOV,
        0b01 => CMP,
        0b10 => ADD,
        0b11 => SUB,
        _ => unsafe { std::hint::unreachable_unchecked() },
    };
    // Compute result.
    let (result, carry, overflow) = match opcode {
        MOV => (op2, false, false),
        CMP | SUB => alu::calc_sub(op1, op2),
        ADD => alu::calc_add(op1, op2),
        _ => unsafe { std::hint::unreachable_unchecked() },
    };
    // Write back register.
    if !opcode.is_test() {
        s.cpu_reg_set(reg_d, result);
    }
    // Write back condition flags.
    s.cpu.cpsr.cond_flag_z = result == 0;
    s.cpu.cpsr.cond_flag_n = result.bit(31);
    if opcode.is_arithmetic() {
        s.cpu.cpsr.cond_flag_c = carry;
        s.cpu.cpsr.cond_flag_v = overflow;
    }

    InstructionResult::Normal
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
