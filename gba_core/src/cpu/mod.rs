mod alu;
mod arm;
mod cond;
mod exception;
mod psr;
mod thumb;

use crate::bus::MemoryAccessType;
use crate::Gba;
use psr::ProgramStatusRegister;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum CpuExecutionState {
    /// ARM execution state.
    Arm = 0,

    /// Thumb execution state.
    Thumb = 1,
}

const REG_PC: usize = 15;
const REG_LR: usize = 14;
const REG_SP: usize = 13;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum CpuMode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}

impl CpuMode {
    fn bank_index(self) -> usize {
        use CpuMode::*;
        match self {
            User | System => 0,
            Fiq => 1,
            Supervisor => 2,
            Abort => 3,
            Irq => 4,
            Undefined => 5,
        }
    }

    fn is_privileged(self) -> bool {
        self != CpuMode::User
    }

    fn has_spsr(self) -> bool {
        !matches!(self, CpuMode::User | CpuMode::System)
    }

    fn from_u32(value: u32) -> Self {
        match value {
            0b10000 => CpuMode::User,
            0b10001 => CpuMode::Fiq,
            0b10010 => CpuMode::Irq,
            0b10011 => CpuMode::Supervisor,
            0b10111 => CpuMode::Abort,
            0b11011 => CpuMode::Undefined,
            0b11111 => CpuMode::System,
            val @ _ => panic!("Unknown CPU mode 0b{:05b}", val),
        }
    }
}

/// Instruction execution result.
enum InstructionResult {
    /// Regular instruction. Increment PC after.
    Normal,

    /// Jumped to a new PC. Don't increment PC.
    Branch,
}

/// State for the CPU.
pub struct Cpu {
    /// r15: the program counter.
    /// Reflects the instruction currently being *fetched* (not executed).
    pub pc: u32,

    /// The first 15 user general purpose registers, r0 to r14.
    /// r13: stack pointer in THUMB. General register in ARM.
    /// r14: link register
    pub gpr: [u32; 15],

    /// Banked registers for FIQ mode. r8 to r12.
    pub gpr_banked_fiq_r8_r12: [u32; 5],

    /// Old registers for r8 to r12 when we're in FIQ mode.
    pub gpr_banked_old_r8_r12: [u32; 5],

    /// Banked r13 for privileged modes.
    /// _, fiq, svc, abt, irq, und
    pub gpr_banked_r13: [u32; 6],

    /// Banked r14 for privileged modes.
    pub gpr_banked_r14: [u32; 6],

    /// Banked SPSRs.
    pub spsr_banked: [u32; 6],

    /// Saved program status register for the current mode.
    pub spsr: u32,

    /// Current program status register.
    pub cpsr: ProgramStatusRegister,

    /// Instructions working their way through the pipeline.
    /// ARM7TDMI has a 3 stage pipeline: fetch -> decode -> execute.
    /// The instruction in 'fetch' is at index 1.
    /// The instruction in 'decode' is at index 0.
    #[allow(unused)]
    pipeline: [u32; 2],

    /// Next fetch memory access type.
    /// Normally Sequential. Becomes NonSequential if the previous instruction accessed memory.
    next_fetch_access: MemoryAccessType,
}

impl Cpu {
    /// Initial CPU state.
    ///
    /// Based on GBATEK and ARM7TDMI Technical Reference Sheet "Reset" behavior.
    pub fn new() -> Cpu {
        // R14_svc <- PC, SPSR_svc <- CPSR.
        // CSPR is reset (supervisor mode, I/F bits set, T bit cleared)
        // PC is set to 0x0. All other registers are "indeterminate".
        Cpu {
            pc: 0,
            gpr: [0; 15],
            gpr_banked_old_r8_r12: [0; 5],
            gpr_banked_fiq_r8_r12: [0; 5],
            gpr_banked_r13: [0; 6],
            gpr_banked_r14: [0; 6],
            spsr_banked: [0; 6],
            spsr: 0,
            cpsr: ProgramStatusRegister::new(),
            // Starts filled with 0, which encodes a useless instruction
            // (but not the canonical no-op).
            pipeline: [0; 2],
            next_fetch_access: MemoryAccessType::NonSequential,
        }
    }

    /// Set the CPU state such that it skips the BIOS.
    pub fn skip_bios(&mut self) {
        self.gpr_banked_r13[CpuMode::User.bank_index()] = 0x03007f00;
        self.gpr_banked_r13[CpuMode::Fiq.bank_index()] = 0x03007f00;
        self.gpr_banked_r13[CpuMode::Irq.bank_index()] = 0x03007fa0;
        self.gpr_banked_r13[CpuMode::Supervisor.bank_index()] = 0x03007fe0;
        self.gpr_banked_r13[CpuMode::Abort.bank_index()] = 0x03007f00;
        self.gpr_banked_r13[CpuMode::Undefined.bank_index()] = 0x03007f00;
        self.gpr[REG_SP] = 0x3007f00;
        self.pc = 0x0800_0000;
        self.cpsr = ProgramStatusRegister {
            cond_flag_n: false,
            cond_flag_z: false,
            cond_flag_c: false,
            cond_flag_v: false,
            interrupt_i: false,
            interrupt_f: false,
            execution_state: CpuExecutionState::Arm,
            mode: CpuMode::System,
        };
    }
}

impl Gba {
    /// Do a single CPU emulation step (not necessarily a single clock cycle).
    pub(crate) fn cpu_step(&mut self) {
        // Pump the pipeline.
        let inst = self.cpu.pipeline[0];
        self.cpu.pipeline[0] = self.cpu.pipeline[1];

        match self.cpu.cpsr.execution_state {
            CpuExecutionState::Thumb => {
                let inst = inst as u16;
                // eprint!("\n\n{}", self.cpu_format_debug());
                /*eprintln!(
                    "CPU [Thumb]: PC={:08X}, opcode={:04X}",
                    self.cpu_thumb_pc(),
                    inst
                );*/
                self.cpu.pipeline[1] =
                    self.cpu_load16(self.cpu.pc, self.cpu.next_fetch_access) as u32;

                match self.cpu_execute_thumb(inst) {
                    InstructionResult::Normal => {
                        // Advance program counter.
                        self.cpu.pc += 2;
                        self.cpu.next_fetch_access = MemoryAccessType::Sequential;
                    }
                    InstructionResult::Branch => {}
                }
            }
            CpuExecutionState::Arm => {
                // eprint!("\n\n{}", self.cpu_format_debug());
                /*eprintln!(
                    "CPU [ ARM ]: PC={:08X}, opcode={:08X}",
                    self.cpu_arm_pc(),
                    inst
                );*/
                self.cpu.pipeline[1] = self.cpu_load32(self.cpu.pc, self.cpu.next_fetch_access);

                match self.cpu_execute_arm(inst) {
                    InstructionResult::Normal => {
                        // Advance program counter.
                        self.cpu.pc += 4;
                        self.cpu.next_fetch_access = MemoryAccessType::Sequential;
                    }
                    InstructionResult::Branch => {}
                }
            }
        }
    }

    /// Jump to the given address (and flush the pipeline).
    fn cpu_jump(&mut self, pc: u32) {
        // XXX: consider splitting this into a jump_arm and a jump_thumb.
        match self.cpu.cpsr.execution_state {
            CpuExecutionState::Thumb => {
                let pc = pc & !0b1;
                self.cpu.pipeline[0] = self.cpu_load16(pc, MemoryAccessType::NonSequential) as u32;
                self.cpu.pipeline[1] = self.cpu_load16(pc + 2, MemoryAccessType::Sequential) as u32;
                self.cpu.pc = pc + 4;
            }
            CpuExecutionState::Arm => {
                let pc = pc & !0b11;
                self.cpu.pipeline[0] = self.cpu_load32(pc, MemoryAccessType::NonSequential);
                self.cpu.pipeline[1] = self.cpu_load32(pc + 4, MemoryAccessType::Sequential);
                self.cpu.pc = pc + 8;
            }
        }
        self.cpu.next_fetch_access = MemoryAccessType::Sequential;
    }

    /// Set a register.
    fn cpu_reg_set(&mut self, register: usize, value: u32) {
        match register {
            0..=14 => self.cpu.gpr[register] = value,
            15 => self.cpu_jump(value),
            _ => panic!("Invalid register {}", register),
        }
    }

    /// Get a register.
    fn cpu_reg_get(&self, register: usize) -> u32 {
        match register {
            0..=14 => self.cpu.gpr[register],
            15 => self.cpu.pc,
            _ => panic!("Invalid register {}", register),
        }
    }

    /// Get the current CPU mode.
    #[inline(always)]
    fn cpu_mode(&self) -> CpuMode {
        self.cpu.cpsr.mode
    }

    /// Set the CPU mode, rebanking registers as necessary.
    fn cpu_set_mode(&mut self, new_mode: CpuMode) {
        let old_mode = self.cpu_mode();
        self.cpu.cpsr.mode = new_mode;
        let old_index = old_mode.bank_index();
        let new_index = new_mode.bank_index();
        if old_mode == new_mode || old_index == new_index {
            return;
        }

        // First save banked registers of old mode...
        self.cpu.spsr_banked[old_index] = self.cpu.spsr;
        self.cpu.gpr_banked_r13[old_index] = self.cpu.gpr[13];
        self.cpu.gpr_banked_r14[old_index] = self.cpu.gpr[14];

        // Then restore the registers from the new mode.
        self.cpu.spsr = self.cpu.spsr_banked[new_index];
        self.cpu.gpr[13] = self.cpu.gpr_banked_r13[new_index];
        self.cpu.gpr[14] = self.cpu.gpr_banked_r14[new_index];

        // Switching out of FIQ: restore the non-FIQ r8-r12.
        if old_mode == CpuMode::Fiq {
            for i in 0..5 {
                self.cpu.gpr_banked_fiq_r8_r12[i] = self.cpu.gpr[8 + i];
                self.cpu.gpr[8 + i] = self.cpu.gpr_banked_old_r8_r12[i];
            }
        }

        // Switching into FIQ: restore the FIQ r8-r12.
        if new_mode == CpuMode::Fiq {
            for i in 0..5 {
                self.cpu.gpr_banked_old_r8_r12[i] = self.cpu.gpr[8 + i];
                self.cpu.gpr[8 + i] = self.cpu.gpr_banked_fiq_r8_r12[i];
            }
        }
    }

    /// Do a CPU internal cycle.
    fn cpu_internal_cycle(&mut self) {
        // TODO implement this
    }

    /// Format a debug dump of the CPU.
    pub(crate) fn cpu_format_debug(&self) -> String {
        use std::fmt::Write;
        let mut s = String::new();
        for reg in 0..16 {
            let value = self.cpu_reg_get(reg);
            write!(s, "r{:02}: {:08X}  ", reg, value).unwrap();
            if reg % 4 == 3 {
                writeln!(s).unwrap();
            }
        }
        let cspr: u32 = self.cpu.cpsr.into();
        writeln!(s, "cpsr: {:08X}", cspr).unwrap();
        s
    }
}
