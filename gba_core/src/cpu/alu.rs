use bit::BitIndex;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AluOpcode {
    AND = 0x0, // logical and
    EOR = 0x1, // logical or
    SUB = 0x2, // subtract
    RSB = 0x3, // subtract reversed
    ADD = 0x4, // add
    ADC = 0x5, // add with carry
    SBC = 0x6, // subtract with carry
    RSC = 0x7, // subtract with carry reversed
    TST = 0x8, // test
    TEQ = 0x9, // test exclusive
    CMP = 0xA, // compare
    CMN = 0xB, // compare negative
    ORR = 0xC, // logical OR
    MOV = 0xD, // move
    BIC = 0xE, // bit clear
    MVN = 0xF, // move not
}

impl AluOpcode {
    pub const fn from_u32(value: u32) -> AluOpcode {
        use AluOpcode::*;
        match value & 0xF {
            0x0 => AND,
            0x1 => EOR,
            0x2 => SUB,
            0x3 => RSB,
            0x4 => ADD,
            0x5 => ADC,
            0x6 => SBC,
            0x7 => RSC,
            0x8 => TST,
            0x9 => TEQ,
            0xA => CMP,
            0xB => CMN,
            0xC => ORR,
            0xD => MOV,
            0xE => BIC,
            0xF => MVN,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    pub const fn is_logical(self) -> bool {
        use AluOpcode::*;
        matches!(self, AND | EOR | TST | TEQ | ORR | MOV | BIC | MVN)
    }

    #[allow(unused)]
    pub const fn is_arithmetic(self) -> bool {
        !self.is_logical()
    }

    /// Whether this opcode tests and sets flags but doesn't set a register.
    pub const fn is_test(self) -> bool {
        use AluOpcode::*;
        matches!(self, TST | TEQ | CMP | CMN)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ThumbAluOpcode {
    AND = 0x0, // logical and
    EOR = 0x1, // logical or
    LSL = 0x2, // logical shift left
    LSR = 0x3, // logical shift right
    ASR = 0x4, // arithmetic shift right
    ADC = 0x5, // add with carry
    SBC = 0x6, // subtract with carry
    ROR = 0x7, // rotate right
    TST = 0x8, // test
    NEG = 0x9, // negate
    CMP = 0xA, // compare
    CMN = 0xB, // compare negative
    ORR = 0xC, // logical OR
    MUL = 0xD, // multiply
    BIC = 0xE, // bit clear
    MVN = 0xF, // move not
}

impl ThumbAluOpcode {
    pub const fn from_u16(value: u16) -> ThumbAluOpcode {
        use ThumbAluOpcode::*;
        match value & 0xF {
            0x0 => AND,
            0x1 => EOR,
            0x2 => LSL,
            0x3 => LSR,
            0x4 => ASR,
            0x5 => ADC,
            0x6 => SBC,
            0x7 => ROR,
            0x8 => TST,
            0x9 => NEG,
            0xA => CMP,
            0xB => CMN,
            0xC => ORR,
            0xD => MUL,
            0xE => BIC,
            0xF => MVN,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// Whether this opcode tests and sets flags but doesn't set a register.
    pub const fn is_test(self) -> bool {
        use ThumbAluOpcode::*;
        matches!(self, TST | CMP | CMN)
    }
}

/// Does the ALU "add" operation, returning (result, carry, overflow).
pub fn calc_add(op1: u32, op2: u32) -> (u32, bool, bool) {
    let result = op1.wrapping_add(op2);
    let carry = (op1 as u64).wrapping_add(op2 as u64) > 0xffffffff;
    let overflow = (op1 as i32).overflowing_add(op2 as i32).1;
    (result, carry, overflow)
}

/// Does the ALU "sub" operation, returning (result, carry, overflow).
pub fn calc_sub(op1: u32, op2: u32) -> (u32, bool, bool) {
    let result = op1.wrapping_sub(op2);
    let carry = op2 <= op1;
    let overflow = (op1 as i32).overflowing_sub(op2 as i32).1;
    (result, carry, overflow)
}

/// Does the ALU "adc" operation, returning (result, carry, overflow).
pub fn calc_adc(op1: u32, op2: u32, carry: bool) -> (u32, bool, bool) {
    let result = (op1 as u64) + (op2 as u64) + (carry as u64);
    let carry = result > 0xffffffff;
    let overflow = (!(op1 ^ op2) & (op2 ^ (result as u32))).bit(31);
    (result as u32, carry, overflow)
}

/// Does the ALU "sbc" operation, returning (result, carry, overflow).
pub fn calc_sbc(op1: u32, op2: u32, carry: bool) -> (u32, bool, bool) {
    calc_adc(op1, !op2, carry)
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AluShiftType {
    /// Logical shift left.
    LSL = 0b00,
    /// Logical shift right.
    LSR = 0b01,
    /// Arithmetic shift right.
    ASR = 0b10,
    /// Rotate right.
    ROR = 0b11,
}

impl AluShiftType {
    pub const fn from_u32(value: u32) -> AluShiftType {
        use AluShiftType::*;
        match value & 0b11 {
            0b00 => LSL,
            0b01 => LSR,
            0b10 => ASR,
            0b11 => ROR,
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

/// Calculate the number of internal cycles required to multiply by the operand.
pub fn multiply_internal_cycles(operand: u32) -> u32 {
    // From the ARM7TDMI-S technical reference manual:
    // 1 if bits[32:8] are all zero or one. -- 24
    // 2 if bits[32:16] are all zero or one. -- 16
    // 3 if bits[31:24] are all zero or one. -- 8
    // 4 otherwise. -- 0
    let leading_same = u32::max(operand.leading_ones(), operand.leading_zeros());
    4 - (leading_same / 8)
}

/// Shift a 32-bit operand by an 11 bit immediate.
/// Outputs the value and the carry.
pub fn shift_by_immediate(
    shift: AluShiftType,
    operand: u32,
    shift_amount: usize,
    carry_in: bool,
) -> (u32, bool) {
    use AluShiftType::*;
    match shift {
        LSL => {
            // ARM ARM 5.1.5
            if shift_amount == 0 {
                (operand, carry_in)
            } else {
                (operand << shift_amount, operand.bit(32 - shift_amount))
            }
        }
        LSR => {
            // ARM ARM 5.1.7
            if shift_amount == 0 {
                // Treated as shift_amount = 32
                (0, operand.bit(31))
            } else {
                (operand >> shift_amount, operand.bit(shift_amount - 1))
            }
        }
        ASR => {
            // ARM ARM 5.1.9
            if shift_amount == 0 {
                if !operand.bit(31) {
                    (0, operand.bit(31))
                } else {
                    (0xFFFFFFFF, operand.bit(31))
                }
            } else {
                (
                    ((operand as i32) >> shift_amount) as u32,
                    operand.bit(shift_amount - 1),
                )
            }
        }
        ROR => {
            // ARM ARM 5.1.11, 5.1.13
            if shift_amount == 0 {
                // RRX: rotate right with extend (5.1.13)
                (((carry_in as u32) << 31) | (operand >> 1), operand.bit(0))
            } else {
                (
                    operand.rotate_right(shift_amount as u32),
                    operand.bit(shift_amount - 1),
                )
            }
        }
    }
}

/// Shift a 32-bit operand by a value loaded from a register.
/// Outputs the result and the carry.
pub fn shift_by_register(
    shift: AluShiftType,
    operand: u32,
    shift_amount: u32,
    carry_in: bool,
) -> (u32, bool) {
    let shift_amount = shift_amount as usize;
    use AluShiftType::*;
    match shift {
        LSL => {
            // ARM ARM 5.1.6
            if shift_amount == 0 {
                (operand, carry_in)
            } else if shift_amount < 32 {
                (operand << shift_amount, operand.bit(32 - shift_amount))
            } else if shift_amount == 32 {
                (0, operand.bit(0))
            } else {
                (0, false)
            }
        }
        LSR => {
            // ARM ARM 5.1.8
            if shift_amount == 0 {
                (operand, carry_in)
            } else if shift_amount < 32 {
                (operand >> shift_amount, operand.bit(shift_amount - 1))
            } else if shift_amount == 32 {
                (0, operand.bit(31))
            } else {
                (0, false)
            }
        }
        ASR => {
            // ARM ARM 5.1.10
            if shift_amount == 0 {
                (operand, carry_in)
            } else if shift_amount < 32 {
                (
                    ((operand as i32) >> shift_amount) as u32,
                    operand.bit(shift_amount - 1),
                )
            } else if !operand.bit(31) {
                (0, operand.bit(31))
            } else {
                (0xFFFFFFFF, operand.bit(31))
            }
        }
        ROR => {
            // ARM ARM 5.1.12
            let new_amount = shift_amount & 0x1F;
            if shift_amount == 0 {
                (operand, carry_in)
            } else if new_amount == 0 {
                (operand, operand.bit(31))
            } else {
                (
                    operand.rotate_right(new_amount as u32),
                    operand.bit(new_amount - 1),
                )
            }
        }
    }
}
