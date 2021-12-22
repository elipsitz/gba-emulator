use super::{
    alu::{self, ThumbAluOpcode},
    cond::Condition,
    exception::ExceptionType,
    CpuExecutionState, Gba, InstructionResult, REG_LR, REG_PC, REG_SP,
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

// THUMB.1: shift by immediate
fn thumb_exec_shift_imm<const OPCODE: u16>(s: &mut Gba, inst: u16) -> InstructionResult {
    let reg_d = inst.bit_range(0..3) as usize;
    let reg_m = inst.bit_range(3..6) as usize;
    let immediate = inst.bit_range(6..11) as usize; // 5 bit immediate

    let shift_type = alu::AluShiftType::from_u32(OPCODE as u32);
    let operand = s.cpu_reg_get(reg_m);
    let (result, shift_carry) =
        alu::shift_by_immediate(shift_type, operand, immediate, s.cpu.cpsr.cond_flag_c);

    s.cpu_reg_set(reg_d, result);
    s.cpu.cpsr.cond_flag_c = shift_carry;
    s.cpu.cpsr.cond_flag_z = result == 0;
    s.cpu.cpsr.cond_flag_n = result.bit(31);
    InstructionResult::Normal
}

// THUMB.2: add / subtract
fn thumb_exec_add_sub<const IMM: bool, const SUB: bool>(
    s: &mut Gba,
    inst: u16,
) -> InstructionResult {
    let reg_d = inst.bit_range(0..3) as usize;
    let reg_n = inst.bit_range(3..6) as usize;
    let op1 = s.cpu_reg_get(reg_n);
    let op2 = if IMM {
        inst.bit_range(6..9) as u32
    } else {
        let reg_m = inst.bit_range(6..9) as usize;
        s.cpu_reg_get(reg_m)
    };

    let (result, carry, overflow) = if SUB {
        alu::calc_sub(op1, op2)
    } else {
        alu::calc_add(op1, op2)
    };

    s.cpu_reg_set(reg_d, result);
    s.cpu.cpsr.cond_flag_n = result.bit(31);
    s.cpu.cpsr.cond_flag_z = result == 0;
    s.cpu.cpsr.cond_flag_c = carry;
    s.cpu.cpsr.cond_flag_v = overflow;
    InstructionResult::Normal
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

// THUMB.5: data-processing register
fn thumb_exec_alu_register<const OPCODE: u16>(s: &mut Gba, inst: u16) -> InstructionResult {
    let reg_d = inst.bit_range(0..3) as usize;
    let reg_m = inst.bit_range(3..6) as usize;
    let opcode = ThumbAluOpcode::from_u16(OPCODE);

    let op1 = s.cpu_reg_get(reg_d);
    let op2 = s.cpu_reg_get(reg_m);

    let carry_in = s.cpu.cpsr.cond_flag_c;
    let overflow_in = s.cpu.cpsr.cond_flag_v;
    use alu::AluShiftType;
    use ThumbAluOpcode::*;
    let (result, carry, overflow) = match opcode {
        AND | TST => (op1 & op2, carry_in, overflow_in),
        EOR => (op1 ^ op2, carry_in, overflow_in),
        //orr, bic, mvn
        LSL => {
            s.cpu_internal_cycle();
            let (result, carry) = alu::shift_by_register(AluShiftType::LSL, op1, op2, carry_in);
            (result, carry, overflow_in)
        }
        LSR => {
            s.cpu_internal_cycle();
            let (result, carry) = alu::shift_by_register(AluShiftType::LSR, op1, op2, carry_in);
            (result, carry, overflow_in)
        }
        ASR => {
            s.cpu_internal_cycle();
            let (result, carry) = alu::shift_by_register(AluShiftType::ASR, op1, op2, carry_in);
            (result, carry, overflow_in)
        }
        ADC => alu::calc_adc(op1, op2, carry_in),
        SBC => alu::calc_sbc(op1, op2, carry_in),
        ROR => {
            s.cpu_internal_cycle();
            let (result, carry) = alu::shift_by_register(AluShiftType::ROR, op1, op2, carry_in);
            (result, carry, overflow_in)
        }
        NEG => alu::calc_sub(0, op2),
        CMP => alu::calc_sub(op1, op2),
        CMN => alu::calc_add(op1, op2),
        ORR => (op1 | op2, carry_in, overflow_in),
        MUL => {
            let num_internal_cycles = alu::multiply_internal_cycles(op2);
            for _ in 0..num_internal_cycles {
                s.cpu_internal_cycle();
            }
            (op1.wrapping_mul(op2), carry_in, overflow_in)
        }
        BIC => (op1 & (!op2), carry_in, overflow_in),
        MVN => (!op2, carry_in, overflow_in),
    };

    // Write flags and set output register.
    s.cpu.cpsr.cond_flag_z = result == 0;
    s.cpu.cpsr.cond_flag_n = result.bit(31);
    s.cpu.cpsr.cond_flag_c = carry;
    s.cpu.cpsr.cond_flag_v = overflow;
    if !opcode.is_test() {
        s.cpu_reg_set(reg_d, result);
    }

    InstructionResult::Normal
}

// THUMB.5: Hi register operations
fn thumb_exec_hireg<const OPCODE: u16, const MSB_REG_D: bool, const MSB_REG_S: bool>(
    s: &mut Gba,
    inst: u16,
) -> InstructionResult {
    let reg_s = inst.bit_range(3..7) as usize;
    let reg_d = (inst.bit_range(0..3) as usize) | ((MSB_REG_D as usize) << 3);
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

// THUMB.13: add offset to stack pointer
fn thumb_exec_adjust_sp<const SUB: bool>(s: &mut Gba, inst: u16) -> InstructionResult {
    let immed = inst.bit_range(0..7) as u32;
    let offset = immed * 4;

    let sp = s.cpu_reg_get(REG_SP);
    let new_sp = if SUB {
        sp.wrapping_sub(offset)
    } else {
        sp.wrapping_add(offset)
    };
    s.cpu_reg_set(REG_SP, new_sp);
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

// THUMB.18: unconditional branch
fn thumb_exec_branch(s: &mut Gba, inst: u16) -> InstructionResult {
    let immediate = (inst.bit_range(0..11) << 1) as u32;
    let offset = if immediate.bit(11) {
        immediate | 0b1111_1111_1111_1111_1111_0000_0000_0000
    } else {
        immediate
    };
    let pc = s.cpu_reg_get(REG_PC);
    let new_pc = pc.wrapping_add(offset);
    s.cpu_reg_set(REG_PC, new_pc);
    InstructionResult::Branch
}

// THUMB.19: branch and link
fn thumb_exec_branch_link<const SUFFIX: bool>(s: &mut Gba, inst: u16) -> InstructionResult {
    let immediate = (inst.bit_range(0..11)) as u32;
    dbg!(SUFFIX, immediate);
    if SUFFIX {
        // Second instruction.
        let new_pc = s.cpu_reg_get(REG_LR) + (immediate << 1);
        let return_address = (s.cpu_thumb_pc() + 2) | 1;
        s.cpu_reg_set(REG_PC, new_pc);
        s.cpu_reg_set(REG_LR, return_address);
        InstructionResult::Branch
    } else {
        // First instruction.
        let signed_offset = if immediate.bit(10) {
            immediate | 0b1111_1111_1111_1111_1111_1000_0000_0000
        } else {
            immediate
        };
        let offset = signed_offset << 12;
        let pc = s.cpu_reg_get(REG_PC);
        let output = pc.wrapping_add(offset);
        s.cpu_reg_set(REG_LR, output);
        InstructionResult::Normal
    }
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
