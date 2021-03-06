use super::{
    alu,
    cond::Condition,
    exception::ExceptionType,
    CpuExecutionState, InstructionResult,
    MemoryAccessType::{self, *},
    REG_LR, REG_PC,
};
use crate::{
    cpu::{psr::ProgramStatusRegister, CpuMode},
    Gba,
};

use bit::BitIndex;

/*
 |..3 ..................2 ..................1 ..................0|
 |1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0_9_8_7_6_5_4_3_2_1_0|
 |_Cond__|0_0_0|___Op__|S|__Rn___|__Rd___|__Shift__|Typ|0|__Rm___| DataProc
 |_Cond__|0_0_0|___Op__|S|__Rn___|__Rd___|__Rs___|0|Typ|1|__Rm___| DataProc
 |_Cond__|0_0_1|___Op__|S|__Rn___|__Rd___|_Shift_|___Immediate___| DataProc
 |_Cond__|0_0_1_1_0|P|1|0|_Field_|__Rd___|_Shift_|___Immediate___| PSR Imm
 |_Cond__|0_0_0_1_0|P|L|0|_Field_|__Rd___|0_0_0_0|0_0_0_0|__Rm___| PSR Reg
 |_Cond__|0_0_0_1_0_0_1_0_1_1_1_1_1_1_1_1_1_1_1_1|0_0|L|1|__Rn___| BX,BLX
 |_Cond__|0_0_0_0_0_0|A|S|__Rd___|__Rn___|__Rs___|1_0_0_1|__Rm___| Multiply
 |_Cond__|0_0_0_0_1|U|A|S|_RdHi__|_RdLo__|__Rs___|1_0_0_1|__Rm___| MulLong
 |_Cond__|0_0_0_1_0|B|0_0|__Rn___|__Rd___|0_0_0_0|1_0_0_1|__Rm___| TransSwp12
 |_Cond__|0_0_0|P|U|0|W|L|__Rn___|__Rd___|0_0_0_0|1|S|H|1|__Rm___| TransReg10
 |_Cond__|0_0_0|P|U|1|W|L|__Rn___|__Rd___|OffsetH|1|S|H|1|OffsetL| TransImm10
 |_Cond__|0_1_0|P|U|B|W|L|__Rn___|__Rd___|_________Offset________| TransImm9
 |_Cond__|0_1_1|P|U|B|W|L|__Rn___|__Rd___|__Shift__|Typ|0|__Rm___| TransReg9
 |_Cond__|0_1_1|________________xxx____________________|1|__xxx__| Undefined
 |_Cond__|1_0_0|P|U|S|W|L|__Rn___|__________Register_List________| BlockTrans
 |_Cond__|1_0_1|L|___________________Offset______________________| B,BL,BLX
 |_Cond__|1_1_0|P|U|N|W|L|__Rn___|__CRd__|__CP#__|____Offset_____| CoDataTrans
 |_Cond__|1_1_1_0|_CPopc_|__CRn__|__CRd__|__CP#__|_CP__|0|__CRm__| CoDataOp
 |_Cond__|1_1_1_0|CPopc|L|__CRn__|__Rd___|__CP#__|_CP__|1|__CRm__| CoRegTrans
 |_Cond__|1_1_1_1|_____________Ignored_by_Processor______________| SWI
*/

/// A function that can execute an ARM instruction.
type ArmHandler = fn(&mut Gba, inst: u32) -> InstructionResult;

/// Dummy unimplemented / invalid ARM instruction.
fn arm_unimplemented(_s: &mut Gba, inst: u32) -> InstructionResult {
    eprintln!(
        "Unknown ARM instruction: {:08x} / {:04b}[{:04b} {:04b}]{:04b}_{:04b}_{:04b}[{:04b}]{:04b}",
        inst,
        (inst >> 28) & 0xf,
        (inst >> 24) & 0xf,
        (inst >> 20) & 0xf,
        (inst >> 16) & 0xf,
        (inst >> 12) & 0xf,
        (inst >> 8) & 0xf,
        (inst >> 4) & 0xf,
        (inst >> 0) & 0xf,
    );
    InstructionResult::Normal
}

/// Branch, branch-and-link.
fn arm_exec_branch<const LINK: bool>(s: &mut Gba, inst: u32) -> InstructionResult {
    // Current PC is actually PC + 8 due to pipeline.
    let offset = ((inst.bit_range(0..24) << 8) as i32) >> 6;
    let pc = ((s.cpu.pc as i32) + offset) as u32;

    if LINK {
        s.cpu_reg_set(REG_LR, s.cpu_arm_pc() + 4);
    }
    s.cpu_reg_set(REG_PC, pc);
    InstructionResult::Branch
}

/// DataProc (ALU).
fn arm_exec_alu<
    const OPCODE: u32,
    const IMM: bool,
    const SETCOND: bool,
    const SHIFT_TYPE: u32,
    const REGSHIFT: bool,
>(
    s: &mut Gba,
    inst: u32,
) -> InstructionResult {
    let reg_n = inst.bit_range(16..20) as usize;
    let reg_d = inst.bit_range(12..16) as usize;
    let op1 = if reg_n == REG_PC {
        if REGSHIFT {
            s.cpu.pc + 4
        } else {
            s.cpu.pc
        }
    } else {
        s.cpu_reg_get(reg_n)
    };

    let carry_flag = s.cpu.cpsr.cond_flag_c;
    let (op2, shift_carry) = if IMM {
        // ARM ARM 5.1.3: Data processing operands - Immediate
        // op2 is a 32-bit immediate: 8 bit value rotated right by 2 * shift amount.
        let immed_8 = inst.bit_range(0..8);
        let rotate_imm = inst.bit_range(8..12);
        let op2 = immed_8.rotate_right(2 * rotate_imm);
        let shift_carry = if rotate_imm == 0 {
            carry_flag
        } else {
            op2.bit(31)
        };
        (op2, shift_carry)
    } else {
        let reg_idx_m = inst.bit_range(0..4) as usize;
        let mut reg_m = s.cpu_reg_get(reg_idx_m);
        let shift_type = alu::AluShiftType::from_u32(SHIFT_TYPE); // bits 5 and 6
        if REGSHIFT {
            // bit 4
            // op2 is a value in a register, shifted by a value in another register.
            // Takes an extra internal cycle.
            s.cpu_internal_cycle();
            let reg_s = inst.bit_range(8..12) as usize;
            let reg_s = s.cpu_reg_get(reg_s) & 0xFF;
            if reg_idx_m == REG_PC {
                reg_m += 4; // Some prefetching behavior.
            }
            alu::shift_by_register(shift_type, reg_m, reg_s, carry_flag)
        } else {
            // op2 is a value in a register, shifted by an immediate value.
            let shift_imm = inst.bit_range(7..12) as usize;
            alu::shift_by_immediate(shift_type, reg_m, shift_imm, carry_flag)
        }
    };

    // Do the actual computation.
    use alu::AluOpcode::*;
    let opcode = alu::AluOpcode::from_u32(OPCODE);
    let (result, carry, overflow) = match opcode {
        AND | TST => (op1 & op2, false, false),
        EOR | TEQ => (op1 ^ op2, false, false),
        SUB | CMP => alu::calc_sub(op1, op2),
        RSB => alu::calc_sub(op2, op1),
        ADD | CMN => alu::calc_add(op1, op2),
        ADC => alu::calc_adc(op1, op2, s.cpu.cpsr.cond_flag_c),
        SBC => alu::calc_sbc(op1, op2, s.cpu.cpsr.cond_flag_c),
        RSC => alu::calc_sbc(op2, op1, s.cpu.cpsr.cond_flag_c),
        ORR => (op1 | op2, false, false),
        MOV => (op2, false, false),
        BIC => (op1 & (!op2), false, false),
        MVN => (!op2, false, false),
    };

    // Write condition flags to CSPR.
    if SETCOND {
        if opcode.is_logical() {
            s.cpu.cpsr.cond_flag_c = shift_carry;
            s.cpu.cpsr.cond_flag_z = result == 0;
            s.cpu.cpsr.cond_flag_n = result.bit(31);
        } else {
            s.cpu.cpsr.cond_flag_c = carry;
            s.cpu.cpsr.cond_flag_v = overflow;
            s.cpu.cpsr.cond_flag_z = result == 0;
            s.cpu.cpsr.cond_flag_n = result.bit(31);
        }

        if reg_d == REG_PC && s.cpu_mode().has_spsr() {
            s.restore_spsr();
        }
    }

    // Write result to register (if not a test instruction).
    if !opcode.is_test() {
        s.cpu_reg_set(reg_d, result);
        if reg_d == REG_PC {
            return InstructionResult::Branch;
        }
    }

    InstructionResult::Normal
}

/// Load and store word or unsigned byte.
fn arm_exec_ldr_str_word_byte<
    const IMMEDIATE: bool,
    const PREINDEX: bool,
    const UP: bool,
    const BYTE: bool,
    const WRITEBACK: bool,
    const LOAD: bool,
    const SHIFT_TYPE: u32,
>(
    s: &mut Gba,
    inst: u32,
) -> InstructionResult {
    // IMMEDIATE, PREINDEX, U, WRITEBACK => used by addressing mode
    // LOAD = 1 -> load; L = 0 -> store
    // BYTE = 1 -> unsigned byte; BYTE = 0 -> word
    let reg_d = inst.bit_range(12..16) as usize; // load/store register
    let reg_n = inst.bit_range(16..20) as usize; // addressing register

    if !PREINDEX && WRITEBACK {
        todo!("Handle LDR/STR [T] instructions");
    }

    let offset = if !IMMEDIATE {
        // IMMEDIATE=0 actually means... use the immediate.
        inst.bit_range(0..12)
    } else {
        let shift_imm = inst.bit_range(7..12);
        let reg_m = inst.bit_range(0..4) as usize;
        let reg_m = s.cpu_reg_get(reg_m);
        let shift_type = alu::AluShiftType::from_u32(SHIFT_TYPE); // bits 5 and 6

        use alu::AluShiftType::*;
        match shift_type {
            LSL => reg_m << shift_imm,
            LSR => {
                if shift_imm == 0 {
                    0
                } else {
                    reg_m >> shift_imm
                }
            }
            ASR => {
                if shift_imm == 0 {
                    if reg_m.bit(31) {
                        0xFFFFFFFF
                    } else {
                        0
                    }
                } else {
                    ((reg_m as i32) >> shift_imm) as u32
                }
            }
            ROR => {
                if shift_imm == 0 {
                    // RRX
                    let carry = s.cpu.cpsr.cond_flag_c as u32;
                    (carry << 31) | (reg_m >> 1)
                } else {
                    reg_m.rotate_right(shift_imm)
                }
            }
        }
    };

    let base = s.cpu_reg_get(reg_n);
    let computed = if UP {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };

    let address = if PREINDEX { computed } else { base };
    if LOAD {
        let data = if BYTE {
            s.cpu_load8(address, MemoryAccessType::NonSequential) as u32
        } else {
            let value = s.cpu_load32(address & !0b11, MemoryAccessType::NonSequential);
            // XXX: is this supposed to involve the carry flag at all?
            value.rotate_right(8 * (address & 0b11))
        };
        s.cpu_internal_cycle();
        s.cpu_reg_set(reg_d, data);
    } else {
        let mut data = s.cpu_reg_get(reg_d);
        if reg_d == REG_PC {
            // On GBA's ARM7TDMI-S, storing PC stores PC + 12 (so add 4 here).
            data += 4;
        }

        if BYTE {
            s.cpu_store8(
                address,
                (data & 0xFF) as u8,
                MemoryAccessType::NonSequential,
            );
        } else {
            // STR instructions ignore the least significant two bits of address.
            s.cpu_store32(address & !0b11, data, MemoryAccessType::NonSequential);
        }
    }

    if (WRITEBACK || !PREINDEX) && (!LOAD || reg_d != reg_n) {
        s.cpu_reg_set(reg_n, computed);
    }

    s.cpu.next_fetch_access = NonSequential;
    if LOAD && reg_d == REG_PC {
        InstructionResult::Branch
    } else {
        InstructionResult::Normal
    }
}

/// Move PSR register to GP register.
fn arm_exec_mrs<const USE_SPSR: bool>(s: &mut Gba, inst: u32) -> InstructionResult {
    let reg_d = inst.bit_range(12..16) as usize;
    let value: u32 = if USE_SPSR {
        // XXX: what if we don't have a SPSR in this mode?
        s.cpu.spsr
    } else {
        s.cpu.cpsr.into()
    };
    if reg_d != REG_PC {
        s.cpu_reg_set(reg_d, value);
    }
    InstructionResult::Normal
}

/// Move GP register or immediate to status register.
fn arm_exec_msr<const USE_SPSR: bool, const IMMEDIATE: bool>(
    s: &mut Gba,
    inst: u32,
) -> InstructionResult {
    let field_c = inst.bit(16);
    #[allow(unused)]
    let field_x = inst.bit(17);
    #[allow(unused)]
    let field_s = inst.bit(18);
    let field_f = inst.bit(19);
    let operand = if IMMEDIATE {
        // Same as ALU.
        let immed_8 = inst.bit_range(0..8);
        let rotate_imm = inst.bit_range(8..12);
        immed_8.rotate_right(2 * rotate_imm)
    } else {
        let reg_m = inst.bit_range(0..4) as usize;
        s.cpu_reg_get(reg_m)
    };

    let current_mode = s.cpu_mode();
    if USE_SPSR {
        if field_c {
            s.cpu.spsr.set_bit_range(0..8, operand.bit_range(0..8));
        }
        if field_f {
            s.cpu.spsr.set_bit_range(24..32, operand.bit_range(24..32));
        }
    } else {
        if field_c && current_mode.is_privileged() {
            let new_mode = CpuMode::from_u32(operand.bit_range(0..5));
            s.cpu_set_mode(new_mode);
            // From the ARM ARM: The MSR instruction must not be used to alter the T bit in the CPSR.
            // As such, we won't bother setting "execution_state" (ARM vs Thumb, the T bit).
            s.cpu.cpsr.interrupt_f = operand.bit(6);
            s.cpu.cpsr.interrupt_i = operand.bit(7);
        }
        if field_f {
            s.cpu.cpsr.cond_flag_n = operand.bit(31);
            s.cpu.cpsr.cond_flag_z = operand.bit(30);
            s.cpu.cpsr.cond_flag_c = operand.bit(29);
            s.cpu.cpsr.cond_flag_v = operand.bit(28);
        }
    }

    InstructionResult::Normal
}

/// Load/store multiple
fn arm_exec_ldm_stm<
    const PREINDEX: bool,
    const UP: bool,
    const S: bool,
    const WRITEBACK: bool,
    const LOAD: bool,
>(
    s: &mut Gba,
    inst: u32,
) -> InstructionResult {
    let reg_n = inst.bit_range(16..20) as usize;
    let mut reg_list = inst.bit_range(0..16) as usize;
    let base = s.cpu_reg_get(reg_n);

    let mut num_registers = reg_list.count_ones();
    // Weird behavior ("unpredictable" according to the ARM ARM).
    // If the list is empty, it'll transfer r15 only, but decrement/increment as if
    // all registers were transferred.
    if num_registers == 0 {
        reg_list = 1 << 15;
        num_registers = 16;
    }

    let start_address = match (PREINDEX, UP) {
        (false, true) => base,                // Increment after.
        (true, true) => base.wrapping_add(4), // Increment before.
        (false, false) => base.wrapping_sub(4 * num_registers).wrapping_add(4), // Decrement after.
        (true, false) => base.wrapping_sub(4 * num_registers), // Decrement before.
    };
    let start_address = start_address & !0b11;

    if LOAD {
        s.cpu_internal_cycle();
    }

    let previous_mode = s.cpu_mode();
    if S {
        s.cpu_set_mode(CpuMode::User);
    }

    let mut address = start_address;
    let mut access_type = NonSequential;
    let mut first = true;
    for reg in 0..=REG_PC {
        if reg_list.bit(reg) {
            if LOAD {
                let value = s.cpu_load32(address & !0b11, access_type);
                s.cpu_reg_set(reg, value);
                if reg == REG_PC && S {
                    s.restore_spsr();
                }
            } else {
                let mut value = s.cpu_reg_get(reg);
                if reg == REG_PC {
                    // STM actually loads PC + 12 (so add 4 here).
                    value += 4;
                }
                if reg == reg_n && !first {
                    // Storing the address base register.
                    // Store the *new* value.
                    value = if UP {
                        base.wrapping_add(4 * num_registers)
                    } else {
                        base.wrapping_sub(4 * num_registers)
                    };
                }
                s.cpu_store32(address & !0b11, value, access_type);
            }
            first = false;
            address += 4;
            access_type = Sequential;
        }
    }

    if S {
        s.cpu_set_mode(previous_mode);
    }

    if WRITEBACK && !(LOAD && reg_list.bit(reg_n)) {
        // According to GBATEK for ARMv4:
        // Writeback with Rb included in Rlist:
        // STM: Store OLD base if Rb is FIRST entry in Rlist, otherwise store NEW base
        // LDM: no writeback

        let value = if UP {
            base.wrapping_add(4 * num_registers)
        } else {
            base.wrapping_sub(4 * num_registers)
        };
        s.cpu_reg_set(reg_n, value); // XXX: what if we're writing PC?
    }

    if (LOAD && reg_list.bit(REG_PC)) || (reg_n == REG_PC && WRITEBACK) {
        // We wrote the PC so it's a branch.
        InstructionResult::Branch
    } else {
        InstructionResult::Normal
    }
}

/// Branch / exchange instruction set.
fn arm_exec_branch_exchange(s: &mut Gba, inst: u32) -> InstructionResult {
    use CpuExecutionState::*;
    let reg_m = inst.bit_range(0..4) as usize;
    let address = s.cpu_reg_get(reg_m);
    let thumb = address.bit(0);
    s.cpu.cpsr.execution_state = if thumb { Thumb } else { Arm };
    s.cpu_jump(address);
    InstructionResult::Branch
}

fn arm_exec_mul<const ACCUMULATE: bool, const SET_FLAGS: bool>(
    s: &mut Gba,
    inst: u32,
) -> InstructionResult {
    let reg_m = s.cpu_reg_get(inst.bit_range(0..4) as usize);
    let reg_s = s.cpu_reg_get(inst.bit_range(8..12) as usize);
    let reg_n = s.cpu_reg_get(inst.bit_range(12..16) as usize);
    let reg_d = inst.bit_range(16..20) as usize;

    let mut result = reg_m.wrapping_mul(reg_s);
    if ACCUMULATE {
        s.cpu_internal_cycle();
        result = result.wrapping_add(reg_n);
    }
    if SET_FLAGS {
        s.cpu.cpsr.cond_flag_n = result.bit(31);
        s.cpu.cpsr.cond_flag_z = result == 0;
    }
    s.cpu_reg_set(reg_d, result);

    let num_internal_cycles = alu::multiply_internal_cycles(reg_s);
    for _ in 0..num_internal_cycles {
        s.cpu_internal_cycle();
    }

    InstructionResult::Normal
}

fn arm_exec_mul_long<const SIGNED: bool, const ACCUMULATE: bool, const SET_FLAGS: bool>(
    s: &mut Gba,
    inst: u32,
) -> InstructionResult {
    let reg_m = s.cpu_reg_get(inst.bit_range(0..4) as usize);
    let reg_s = s.cpu_reg_get(inst.bit_range(8..12) as usize);
    let reg_lo = inst.bit_range(12..16) as usize;
    let reg_hi = inst.bit_range(16..20) as usize;

    let mut result = if SIGNED {
        (reg_m as i32 as i64).wrapping_mul(reg_s as i32 as i64) as u64
    } else {
        (reg_m as u64).wrapping_mul(reg_s as u64)
    };
    if ACCUMULATE {
        s.cpu_internal_cycle();
        let lo = s.cpu_reg_get(reg_lo) as u64;
        let hi = s.cpu_reg_get(reg_hi) as u64;
        result = result.wrapping_add(hi << 32 | lo);
    }

    s.cpu_reg_set(reg_hi, (result >> 32) as u32);
    s.cpu_reg_set(reg_lo, (result & 0xFFFF_FFFF) as u32);
    if SET_FLAGS {
        s.cpu.cpsr.cond_flag_n = result.bit(63);
        s.cpu.cpsr.cond_flag_z = result == 0;
    }

    s.cpu_internal_cycle(); // Multiply long has an additional cycle.
    let num_internal_cycles = alu::multiply_internal_cycles(reg_s);
    for _ in 0..num_internal_cycles {
        s.cpu_internal_cycle();
    }

    InstructionResult::Normal
}

fn arm_exec_swap<const BYTE: bool>(s: &mut Gba, inst: u32) -> InstructionResult {
    let reg_index_n = inst.bit_range(16..20) as usize;
    let reg_index_d = inst.bit_range(12..16) as usize;
    let reg_index_m = inst.bit_range(0..4) as usize;

    let address = s.cpu_reg_get(reg_index_n);
    let temp = if BYTE {
        s.cpu_load8(address, NonSequential) as u32
    } else {
        let value = s.cpu_load32(address & !0b11, NonSequential);
        value.rotate_right(8 * (address & 0b11))
    };

    if BYTE {
        let store_data = s.cpu_reg_get(reg_index_m) as u8;
        s.cpu_store8(address, store_data, NonSequential);
    } else {
        let store_data = s.cpu_reg_get(reg_index_m);
        s.cpu_store32(address & !0b11, store_data, NonSequential);
    }

    s.cpu_internal_cycle();
    s.cpu_reg_set(reg_index_d, temp);

    if reg_index_d == REG_PC {
        InstructionResult::Branch
    } else {
        InstructionResult::Normal
    }
}

fn arm_exec_ld_st_halfword_byte<
    const PREINDEX: bool,
    const UP: bool,
    const IMMEDIATE: bool,
    const WRITEBACK: bool,
    const LOAD: bool,
    const SIGNED: bool,
    const HALFWORD: bool,
>(
    s: &mut Gba,
    inst: u32,
) -> InstructionResult {
    assert!(!(!SIGNED && !HALFWORD)); // Unsigned byte is not valid.
    assert!(!(SIGNED && !LOAD)); // Signed store is not valid.
    let reg_index_n = inst.bit_range(16..20) as usize;
    let reg_index_d = inst.bit_range(12..16) as usize;

    let offset = if IMMEDIATE {
        (inst.bit_range(8..12) << 4) | inst.bit_range(0..4)
    } else {
        let reg_index_m = inst.bit_range(0..4) as usize;
        s.cpu_reg_get(reg_index_m)
    };

    let base = s.cpu_reg_get(reg_index_n);
    let computed = if UP {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };

    let address = if PREINDEX { computed } else { base };
    if LOAD {
        let data = if HALFWORD {
            // According to ARM ARM, misaligned load is unpredictable.
            let data = s.cpu_load16(address & !0b1, NonSequential);
            if SIGNED {
                let data = data as i16;
                let data = data >> (8 * (address & 0b1));
                data as u32
            } else {
                // In practice, unsigned does the rotation thing like in LDR.
                let data = data as u32;
                data.rotate_right(8 * (address & 0b1))
            }
        } else {
            let data = s.cpu_load8(address, NonSequential);
            data as i8 as u32 // This instruction allows signed byte only.
        };
        s.cpu_internal_cycle();
        s.cpu_reg_set(reg_index_d, data);
    } else {
        debug_assert!(HALFWORD);
        // According to ARM ARM, misaligned store is unpredictable.
        // However, in practice store ignores the least significant bit of the address.
        let data = s.cpu_reg_get(reg_index_d) as u16;
        s.cpu_store16(address & !0b1, data, NonSequential);
    }

    if (WRITEBACK || !PREINDEX) && (!LOAD || reg_index_d != reg_index_n) {
        s.cpu_reg_set(reg_index_n, computed);
    }

    // XXX: if reg_d is REG_PC, it's *unpredictable*.
    s.cpu.next_fetch_access = NonSequential;
    InstructionResult::Normal
}

fn arm_exec_swi(s: &mut Gba, _inst: u32) -> InstructionResult {
    let return_address = s.cpu_arm_pc() + 4;
    s.cpu_exception(ExceptionType::SoftwareInterrupt, return_address);
    InstructionResult::Branch
}

// Include look-up table for instruction handlers.
include!(concat!(env!("OUT_DIR"), "/arm_table.rs"));

impl Gba {
    /// Get the program counter of the *currently executing ARM instruction*.
    pub fn cpu_arm_pc(&self) -> u32 {
        // Go back 2 instructions (because pipelining).
        self.cpu.pc.wrapping_sub(8)
    }

    /// Execute the given ARM instruction.
    pub(super) fn cpu_execute_arm(&mut self, inst: u32) -> InstructionResult {
        let condition: Condition = inst.bit_range(28..32).into();
        if condition.evaluate(self) {
            let key = (((inst >> 16) & 0xff0) | ((inst >> 4) & 0xf)) as usize;
            (ARM_HANDLERS[key])(self, inst)
        } else {
            InstructionResult::Normal
        }
    }

    fn restore_spsr(&mut self) {
        // Copy SPSR to CPSR. Switch mode first (to bank registers).
        let new_cpsr: ProgramStatusRegister = self.cpu.spsr.into();
        self.cpu_set_mode(new_cpsr.mode);
        self.cpu.cpsr = new_cpsr;
    }
}
