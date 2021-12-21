use super::{CpuExecutionState, CpuMode, Gba};

#[derive(Copy, Clone, PartialEq, Debug)]
#[allow(unused)]
pub enum ExceptionType {
    Reset,
    Undefined,
    SoftwareInterrupt,
    PrefetchAbort,
    DataAbort,
    Irq,
    Fiq,
}

impl ExceptionType {
    /// Return the address of the exception vector for this type.
    fn vector(self) -> u32 {
        use ExceptionType::*;
        match self {
            Reset => 0x0000_0000,
            Undefined => 0x0000_0004,
            SoftwareInterrupt => 0x0000_0008,
            PrefetchAbort => 0x0000_000C,
            DataAbort => 0x0000_0010,
            Irq => 0x0000_0018,
            Fiq => 0x0000_001C,
        }
    }

    /// Return the mode of the exception.
    fn mode(self) -> CpuMode {
        use ExceptionType::*;
        match self {
            Reset => CpuMode::Supervisor,
            Undefined => CpuMode::Undefined,
            SoftwareInterrupt => CpuMode::Supervisor,
            PrefetchAbort => CpuMode::Abort,
            DataAbort => CpuMode::Abort,
            Irq => CpuMode::Irq,
            Fiq => CpuMode::Fiq,
        }
    }
}

impl Gba {
    pub(crate) fn cpu_exception(&mut self, kind: ExceptionType, return_address: u32) {
        let new_mode = kind.mode();
        let new_index = new_mode.bank_index();

        self.cpu.gpr_banked_r14[new_index] = return_address;
        self.cpu.spsr_banked[new_index] = self.cpu.cpsr.into();
        self.cpu_set_mode(new_mode);
        self.cpu.cpsr.execution_state = CpuExecutionState::Arm;
        if kind == ExceptionType::Reset || kind == ExceptionType::Fiq {
            self.cpu.cpsr.interrupt_f = true;
        }
        self.cpu.cpsr.interrupt_i = true;
        self.cpu_jump(kind.vector());
    }
}
