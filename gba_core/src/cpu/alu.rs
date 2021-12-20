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
