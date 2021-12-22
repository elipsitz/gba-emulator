use super::{
    alu, cond::Condition, exception::ExceptionType, CpuExecutionState, Gba, InstructionResult,
    REG_PC, REG_SP,
};
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

// THUMB.5: Branch-exchange
fn thumb_exec_bx<const MSB_REG_S: bool>(s: &mut Gba, inst: u16) -> InstructionResult {
    // reg_m in bits 3, 4, 5, and H2 in bit 6.
    use CpuExecutionState::*;
    let reg_m = inst.bit_range(3..7) as usize;
    let address = s.cpu_reg_get(reg_m);
    let thumb = address.bit(0);
    s.cpu.cpsr.execution_state = if thumb { Thumb } else { Arm };
    s.cpu_jump(address);
    InstructionResult::Branch
}

// THUMB.5: Hi register operations
fn thumb_exec_hireg<const OPCODE: u16, const MSB_REG_D: bool, const MSB_REG_S: bool>(
    s: &mut Gba,
    inst: u16,
) -> InstructionResult {
    let reg_s = inst.bit_range(3..7) as usize;
    let reg_d = (inst.bit_range(0..3) as usize) & ((MSB_REG_D as usize) << 3);
    let op1 = s.cpu_reg_get(reg_d);
    let op2 = s.cpu_reg_get(reg_s);

    // Decode operation.
    use alu::AluOpcode::*;
    let opcode = match OPCODE {
        0b00 => ADD,
        0b01 => CMP,
        0b10 => MOV,
        // 0b11 encodes BX and is handled by another function.
        _ => unsafe { std::hint::unreachable_unchecked() },
    };
    // Compute result.
    let (result, carry, overflow) = match opcode {
        ADD => (op1.wrapping_add(op2), false, false),
        CMP => alu::calc_sub(op1, op2),
        MOV => (op2, false, false),
        _ => unsafe { std::hint::unreachable_unchecked() },
    };
    // Write back results.
    if opcode == CMP {
        s.cpu.cpsr.cond_flag_z = result == 0;
        s.cpu.cpsr.cond_flag_n = result.bit(31);
        s.cpu.cpsr.cond_flag_c = carry;
        s.cpu.cpsr.cond_flag_v = overflow;
        InstructionResult::Normal
    } else {
        s.cpu_reg_set(reg_d, result);
        if reg_d == REG_PC {
            InstructionResult::Branch
        } else {
            InstructionResult::Normal
        }
    }
}

// THUMB.12: get relative address
fn thumb_exec_address_calc<const SP: bool>(s: &mut Gba, inst: u16) -> InstructionResult {
    let reg_d = inst.bit_range(8..11) as usize;
    let immed = inst.bit_range(0..8) as u32;

    let base = if SP {
        s.cpu_reg_get(REG_SP)
    } else {
        s.cpu_reg_get(REG_PC) & 0xFFFF_FFFC
    };
    let result = base.wrapping_add(immed * 4);
    s.cpu_reg_set(reg_d, result);
    InstructionResult::Normal
}

// THUMB.16: conditional branch
fn thumb_exec_branch_conditional<const COND: u16>(s: &mut Gba, inst: u16) -> InstructionResult {
    let condition: Condition = (COND as u32).into();
    if condition.evaluate(s) {
        let immed = inst.bit_range(0..8);
        let offset = (immed as i8 as u32) << 1;
        let pc = s.cpu_reg_get(REG_PC);
        let new_pc = pc.wrapping_add(offset);
        s.cpu_reg_set(REG_PC, new_pc);
        InstructionResult::Branch
    } else {
        InstructionResult::Normal
    }
}

// THUMB.17: software interrupt
fn thumb_exec_swi(s: &mut Gba, _inst: u16) -> InstructionResult {
    let return_address = s.cpu_thumb_pc() + 2;
    s.cpu_exception(ExceptionType::SoftwareInterrupt, return_address);
    InstructionResult::Branch
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
