use super::{CpuExecutionState, CpuMode};
use bit::BitIndex;

/// Program status register.
#[derive(Copy, Clone, Debug)]
pub struct ProgramStatusRegister {
    /// Condition flag "negative".
    /// Bit 31 of PSR.
    cond_flag_n: bool,

    /// Condition flag "zero".
    /// Bit 30 of PSR.
    cond_flag_z: bool,

    /// Condition flag "carry".
    /// Bit 29 of PSR.
    cond_flag_c: bool,

    /// Condition flag "overflow".
    /// Bit 28 of PSR.
    cond_flag_v: bool,

    /// Interrupt disable "IRQ".
    /// Bit 7 of PSR.
    interrupt_i: bool,

    /// Interrupt disable "FIQ".
    /// Bit 6 of PSR.
    interrupt_f: bool,

    /// CPU Execution State. ARM or Thumb.
    /// Bit 5 of PSR -- 1 if Thumb, 0 if ARM.
    execution_state: CpuExecutionState,

    /// CPU mode.
    /// Bits 0-4 of PSR.
    mode: CpuMode,
}

impl ProgramStatusRegister {
    /// Initial register state.
    ///
    /// Based on description of "Reset" in GBATEK:
    /// ```Forces PC=VVVV0000h, and forces control bits of CPSR to T=0 (ARM state),
    /// F=1 and I=1 (disable FIQ and IRQ), and M4-0=10011b (Supervisor mode).```
    pub fn new() -> Self {
        ProgramStatusRegister {
            cond_flag_n: false,
            cond_flag_c: false,
            cond_flag_z: false,
            cond_flag_v: false,
            interrupt_i: true,
            interrupt_f: true,
            execution_state: CpuExecutionState::Arm,
            mode: CpuMode::Supervisor,
        }
    }
}

impl Into<u32> for ProgramStatusRegister {
    fn into(self) -> u32 {
        let mut val = 0u32;
        val.set_bit(31, self.cond_flag_n);
        val.set_bit(30, self.cond_flag_c);
        val.set_bit(29, self.cond_flag_z);
        val.set_bit(28, self.cond_flag_v);
        val.set_bit(7, self.interrupt_i);
        val.set_bit(6, self.interrupt_f);
        val.set_bit(5, self.execution_state == CpuExecutionState::Thumb);
        val.set_bit_range(0..5, self.mode as u32);
        val
    }
}

impl From<u32> for ProgramStatusRegister {
    fn from(val: u32) -> Self {
        let mode = match val.bit_range(0..5) {
            super::CPU_MODE_USER => CpuMode::User,
            super::CPU_MODE_FIQ => CpuMode::Fiq,
            super::CPU_MODE_IRQ => CpuMode::Irq,
            super::CPU_MODE_SUPERVISOR => CpuMode::Supervisor,
            super::CPU_MODE_ABORT => CpuMode::Abort,
            super::CPU_MODE_UNDEFINED => CpuMode::Undefined,
            super::CPU_MODE_SYSTEM => CpuMode::System,
            val @ _ => panic!("Unknown CPU mode {:X}", val),
        };

        ProgramStatusRegister {
            cond_flag_n: val.bit(31),
            cond_flag_c: val.bit(30),
            cond_flag_z: val.bit(29),
            cond_flag_v: val.bit(28),
            interrupt_i: val.bit(7),
            interrupt_f: val.bit(6),
            execution_state: if val.bit(5) {
                CpuExecutionState::Thumb
            } else {
                CpuExecutionState::Arm
            },
            mode,
        }
    }
}
