#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Condition {
    /// Equal (Z = 1)
    EQ,

    /// Not equal (Z = 0)
    NE,

    /// Carry set (C = 1)
    CS,

    /// Carry clear (C = 0)
    CC,

    /// Minus (negative) (N = 1)
    MI,

    /// Plus (positive/zero) (N = 0)
    PL,

    /// Overflow (V = 1)
    VS,

    /// No overflow (V = 0)
    VC,

    /// Unsigned higher (C = 1 and Z = 0)
    HI,

    /// Unsigned lower/same (C = 0 or Z = 1)
    LS,

    /// Signed greater-than-equal (N = V)
    GE,

    /// Signed less-than (N != V)
    LT,

    /// Signed greater-than (Z = 0 and N = V)
    GT,

    /// Signed less-than-equal (Z = 1 or N != V)
    LE,

    /// Always (unconditional)
    AL,

    /// Invalid (e.g. 0b1111)
    Invalid,
}

impl From<u32> for Condition {
    fn from(data: u32) -> Self {
        match data {
            0b0000 => Condition::EQ,
            0b0001 => Condition::NE,
            0b0010 => Condition::CS,
            0b0011 => Condition::CC,
            0b0100 => Condition::MI,
            0b0101 => Condition::PL,
            0b0110 => Condition::VS,
            0b0111 => Condition::VC,
            0b1000 => Condition::HI,
            0b1001 => Condition::LS,
            0b1010 => Condition::GE,
            0b1011 => Condition::LT,
            0b1100 => Condition::GT,
            0b1101 => Condition::LE,
            0b1110 => Condition::AL,
            _ => Condition::Invalid,
        }
    }
}

impl Condition {
    pub fn evaluate(self, gba: &crate::Gba) -> bool {
        let cpsr = &gba.cpu.cpsr;
        use Condition::*;
        match self {
            EQ => cpsr.cond_flag_z,
            NE => !cpsr.cond_flag_z,
            CS => cpsr.cond_flag_c,
            CC => !cpsr.cond_flag_c,
            MI => cpsr.cond_flag_n,
            PL => !cpsr.cond_flag_n,
            VS => cpsr.cond_flag_v,
            VC => !cpsr.cond_flag_v,
            HI => cpsr.cond_flag_c && !cpsr.cond_flag_z,
            LS => !cpsr.cond_flag_c || cpsr.cond_flag_z,
            GE => cpsr.cond_flag_n == cpsr.cond_flag_v,
            LT => cpsr.cond_flag_n != cpsr.cond_flag_v,
            GT => !cpsr.cond_flag_z && (cpsr.cond_flag_n == cpsr.cond_flag_v),
            LE => cpsr.cond_flag_z || (cpsr.cond_flag_n != cpsr.cond_flag_v),
            AL => true,
            Invalid => false,
        }
    }
}
