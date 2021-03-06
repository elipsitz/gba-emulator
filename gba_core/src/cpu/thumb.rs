use super::{
    alu::{self, ThumbAluOpcode},
    cond::Condition,
    exception::ExceptionType,
    CpuExecutionState, Gba, InstructionResult,
    MemoryAccessType::*,
    REG_LR, REG_PC, REG_SP,
};
use bit::BitIndex;

/// A function that can execute a Thumb instruction.
type ThumbHandler = fn(&mut Gba, inst: u16) -> InstructionResult;

/// Dummy unimplemented / invalid Thumb instruction.
fn thumb_unimplemented(_s: &mut Gba, inst: u16) -> InstructionResult {
    eprintln!(
        "Unknown Thumb instruction: {:04x} / [{:04b} {:04b} {:02b}] {:02b} {:04b}",
        inst,
        (inst >> 12) & 0b1111,
        (inst >> 8) & 0b1111,
        (inst >> 6) & 0b11,
        (inst >> 4) & 0b11,
        (inst >> 0) & 0b1111,
    );
    InstructionResult::Normal
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

// THUMB.6: load PC-relative
fn thumb_exec_load_pc_relative(s: &mut Gba, inst: u16) -> InstructionResult {
    let immed = inst.bit_range(0..8) as u32;
    let reg_d = inst.bit_range(8..11) as usize;

    let address = (s.cpu.pc & !3).wrapping_add(immed * 4);
    let value = s.cpu_load32(address, NonSequential);
    s.cpu_reg_set(reg_d, value);

    s.cpu_internal_cycle();
    s.cpu.next_fetch_access = NonSequential;
    InstructionResult::Normal
}

// THUMB.7: load/store with register offset
// THUMB.8: load/store sign-extended byte/halfword
fn thumb_exec_ldr_str_reg_offset<const OP: u16>(s: &mut Gba, inst: u16) -> InstructionResult {
    //  __7_|_0___1___0___1_|__Op___|_0_|___Ro______|____Rb_____|____Rd_____|LDR/STR
    //  __8_|_0___1___0___1_|__Op___|_1_|___Ro______|____Rb_____|____Rd_____|""H/SB/SH
    let reg_d = inst.bit_range(0..3) as usize;
    let reg_n = inst.bit_range(3..6) as usize;
    let reg_m = inst.bit_range(6..9) as usize;

    let address = s.cpu_reg_get(reg_n).wrapping_add(s.cpu_reg_get(reg_m));
    let store_val = s.cpu_reg_get(reg_d);
    match OP {
        0b000 => {
            // STR  Rd,[Rb,Ro]   ;store 32bit data  WORD[Rb+Ro] = Rd
            s.cpu_store32(address & !0b11, store_val, NonSequential);
        }
        0b010 => {
            // STRB Rd,[Rb,Ro]   ;store  8bit data  BYTE[Rb+Ro] = Rd
            s.cpu_store8(address, store_val as u8, NonSequential);
        }
        0b100 => {
            // LDR  Rd,[Rb,Ro]   ;load  32bit data  Rd = WORD[Rb+Ro]
            let value = s.cpu_load32(address & !0b11, NonSequential);
            let value = value.rotate_right(8 * (address & 0b11));
            s.cpu_reg_set(reg_d, value);
            s.cpu_internal_cycle();
        }
        0b110 => {
            // LDRB Rd,[Rb,Ro]   ;load   8bit data  Rd = BYTE[Rb+Ro]
            let value = s.cpu_load8(address, NonSequential) as u32;
            s.cpu_reg_set(reg_d, value);
            s.cpu_internal_cycle();
        }
        0b001 => {
            // STRH Rd,[Rb,Ro]  ;store 16bit data          HALFWORD[Rb+Ro] = Rd
            s.cpu_store16(address & !0b1, store_val as u16, NonSequential);
        }
        0b011 => {
            // LDSB Rd,[Rb,Ro]  ;load sign-extended 8bit   Rd = BYTE[Rb+Ro]
            let value = s.cpu_load8(address, NonSequential) as i8 as u32;
            s.cpu_reg_set(reg_d, value);
            s.cpu_internal_cycle();
        }
        0b101 => {
            // LDRH Rd,[Rb,Ro]  ;load zero-extended 16bit  Rd = HALFWORD[Rb+Ro]
            let value = s.cpu_load16(address & !0b1, NonSequential) as u32;
            let value = value.rotate_right(8 * (address & 0b1));
            s.cpu_reg_set(reg_d, value);
            s.cpu_internal_cycle();
        }
        0b111 => {
            // LDSH Rd,[Rb,Ro]  ;load sign-extended 16bit  Rd = HALFWORD[Rb+Ro]
            let value = s.cpu_load16(address & !0b1, NonSequential) as i16;
            let value = (value >> (8 * (address & 0b1))) as u32;
            s.cpu_reg_set(reg_d, value);
            s.cpu_internal_cycle();
        }
        _ => unsafe { std::hint::unreachable_unchecked() },
    }

    s.cpu.next_fetch_access = NonSequential;
    InstructionResult::Normal
}

// THUMB.9 load/store with immediate offset
// THUMB.10 load/store halfword with immediate offset
fn thumb_exec_ldr_str_imm<const BYTE: bool, const HALFWORD: bool, const LOAD: bool>(
    s: &mut Gba,
    inst: u16,
) -> InstructionResult {
    let reg_d = inst.bit_range(0..3) as usize;
    let reg_n = inst.bit_range(3..6) as usize;
    let immed = inst.bit_range(6..11) as u32;

    #[derive(Copy, Clone)]
    enum Width {
        Byte = 1,
        Halfword = 2,
        Word = 4,
    }
    use Width::*;
    let width = if BYTE {
        Byte
    } else if HALFWORD {
        Halfword
    } else {
        Word
    };

    let offset = immed * (width as u32);
    let address = s.cpu_reg_get(reg_n) + offset;
    if LOAD {
        let value = match width {
            Byte => s.cpu_load8(address, NonSequential) as u32,
            Halfword => {
                let value = s.cpu_load16(address & !0b1, NonSequential) as u32;
                value.rotate_right(8 * (address & 0b1))
            }
            Word => {
                let value = s.cpu_load32(address & !0b11, NonSequential);
                value.rotate_right(8 * (address & 0b11))
            }
        };
        s.cpu_reg_set(reg_d, value);
        s.cpu_internal_cycle();
    } else {
        let value = s.cpu_reg_get(reg_d);
        match width {
            Byte => s.cpu_store8(address, value as u8, NonSequential),
            Halfword => s.cpu_store16(address & !0b1, value as u16, NonSequential),
            Word => s.cpu_store32(address & !0b11, value, NonSequential),
        }
    }

    s.cpu.next_fetch_access = NonSequential;
    InstructionResult::Normal
}

// THUMB.11 load/store SP relative
fn thumb_exec_ldr_str_sp<const LOAD: bool>(s: &mut Gba, inst: u16) -> InstructionResult {
    let immed = inst.bit_range(0..8) as u32;
    let reg_d = inst.bit_range(8..11) as usize;

    let address = s.cpu_reg_get(REG_SP).wrapping_add(immed * 4);
    if LOAD {
        let value = s.cpu_load32(address & !0b11, NonSequential);
        let value = value.rotate_right(8 * (address & 0b11));
        s.cpu_reg_set(reg_d, value);
        s.cpu_internal_cycle();
    } else {
        let value = s.cpu_reg_get(reg_d);
        s.cpu_store32(address & !0b11, value, NonSequential);
    }

    s.cpu.next_fetch_access = NonSequential;
    InstructionResult::Normal
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

// THUMB.14: push/pop registers
fn thumb_exec_push_pop<const POP: bool, const PC_LR: bool>(
    s: &mut Gba,
    inst: u16,
) -> InstructionResult {
    let reg_list = inst.bit_range(0..8);
    let num_registers = reg_list.count_ones() + (PC_LR as u32);
    let sp = s.cpu_reg_get(REG_SP);

    if (num_registers == 0) && !PC_LR {
        // Empty rlist: weird behavior, transfer r15 (pc), then change SP by 0x40.
        s.cpu.next_fetch_access = NonSequential;

        return if POP {
            let new_sp = sp.wrapping_add(0x40);
            s.cpu_reg_set(REG_SP, new_sp);

            let value = s.cpu_load32(sp, NonSequential);
            s.cpu_reg_set(REG_PC, value);
            s.cpu_internal_cycle();
            InstructionResult::Branch
        } else {
            let new_sp = sp.wrapping_sub(0x40);
            s.cpu_reg_set(REG_SP, new_sp);

            let value = s.cpu_reg_get(REG_PC);
            s.cpu_store32(new_sp, value, NonSequential);
            InstructionResult::Normal
        };
    }

    let (start_address, new_sp) = if POP {
        let new_sp = sp.wrapping_add(4 * num_registers);
        (sp, new_sp)
    } else {
        let start_address = sp.wrapping_sub(4 * num_registers);
        (start_address, start_address)
    };

    let mut address = start_address;
    let mut access_type = NonSequential;
    for reg in 0..=7 {
        if reg_list.bit(reg) {
            if POP {
                let value = s.cpu_load32(address & !0b11, access_type);
                s.cpu_reg_set(reg, value);
            } else {
                let value = s.cpu_reg_get(reg);
                s.cpu_store32(address & !0b11, value, access_type);
            }

            address += 4;
            access_type = Sequential;
        }
    }

    let mut instruction_result = InstructionResult::Normal;
    if PC_LR {
        if POP {
            // Pop PC.
            let value = s.cpu_load32(address & !0b11, access_type);
            s.cpu_reg_set(REG_PC, value);
            instruction_result = InstructionResult::Branch;
        } else {
            // Store LR.
            let value = s.cpu_reg_get(REG_LR);
            s.cpu_store32(address, value, access_type);
        }
    }

    if POP {
        s.cpu_internal_cycle();
    }
    s.cpu.next_fetch_access = NonSequential;
    s.cpu_reg_set(REG_SP, new_sp);
    instruction_result
}

// THUMB.15: multiple load/store
fn thumb_exec_ldr_str_multiple<const LOAD: bool>(s: &mut Gba, inst: u16) -> InstructionResult {
    let reg_list = inst.bit_range(0..8);
    let reg_n = inst.bit_range(8..11) as usize;
    let num_registers = reg_list.count_ones();

    let start_address = s.cpu_reg_get(reg_n);
    let mut new_address = start_address.wrapping_add(4 * num_registers);

    let instruction_result = if num_registers != 0 {
        let mut address = start_address;
        let mut access_type = NonSequential;
        let mut first = true;
        for reg in 0..=7 {
            if reg_list.bit(reg) {
                if LOAD {
                    let value = s.cpu_load32(address & !0b11, access_type);
                    s.cpu_reg_set(reg, value);
                } else {
                    let mut value = s.cpu_reg_get(reg);
                    if reg == reg_n && !first {
                        // Storing the address base register. Store the *new* value.
                        value = new_address;
                    }
                    s.cpu_store32(address & !0b11, value, access_type);
                }
                first = false;
                address += 4;
                access_type = Sequential;
            }
        }
        InstructionResult::Normal
    } else {
        // Weird behavior ("unpredictable" according to the ARM ARM).
        // If the list is empty, it'll transfer r15 only, but decrement/increment as if
        // all registers were transferred.
        new_address = start_address + 0x40;
        if LOAD {
            let value = s.cpu_load32(start_address & !0b11, NonSequential);
            s.cpu_reg_set(REG_PC, value);
            InstructionResult::Branch
        } else {
            // When storing here we actually store PC + 6... so + 2 from what we read here.
            let value = s.cpu_reg_get(REG_PC) + 2;
            s.cpu_store32(start_address & !0b11, value, NonSequential);
            InstructionResult::Normal
        }
    };

    if LOAD {
        s.cpu_internal_cycle();
    }
    s.cpu.next_fetch_access = NonSequential;
    if !(LOAD && reg_list.bit(reg_n)) {
        // With LOAD, if Rn is in Rlist, the new address is not written back.
        s.cpu_reg_set(reg_n, new_address);
    }
    instruction_result
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
    if SUFFIX {
        // Second instruction.
        let new_pc = s.cpu_reg_get(REG_LR).wrapping_add(immediate << 1);
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
