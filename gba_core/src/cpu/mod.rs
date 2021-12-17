mod arm;
mod cond;
mod psr;

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

const CPU_MODE_USER: u32 = 0b10000;
const CPU_MODE_FIQ: u32 = 0b10001;
const CPU_MODE_IRQ: u32 = 0b10010;
const CPU_MODE_SUPERVISOR: u32 = 0b10011;
const CPU_MODE_ABORT: u32 = 0b10111;
const CPU_MODE_UNDEFINED: u32 = 0b11011;
const CPU_MODE_SYSTEM: u32 = 0b11111;

#[derive(Copy, Clone, Debug)]
#[repr(u32)]
pub enum CpuMode {
    User = CPU_MODE_USER,
    Fiq = CPU_MODE_FIQ,
    Irq = CPU_MODE_IRQ,
    Supervisor = CPU_MODE_SUPERVISOR,
    Abort = CPU_MODE_ABORT,
    Undefined = CPU_MODE_UNDEFINED,
    System = CPU_MODE_SYSTEM,
}

/// Instruction execution result.
enum InstructionResult {
    /// Regular instruction. Increment PC after.
    Normal,

    /// Jumped to a new PC.
    Branch(u32),
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
    pub gpr_banked_fiq: [u32; 5],

    /// Banked r13 for privileged modes (except system).
    /// fiq, svc, abt, irq, und
    pub gpr_banked_r13: [u32; 5],

    /// Banked r14 for privileged modes (except system).
    pub gpr_banked_r14: [u32; 5],

    /// Saved program status register (for the current mode).
    pub spsr: ProgramStatusRegister,

    /// Current program status register.
    pub cpsr: ProgramStatusRegister,

    /// Instructions working their way through the pipeline.
    /// ARM7TDMI has a 3 stage pipeline: fetch -> decode -> execute.
    /// The instruction in 'fetch' is at index 1.
    /// The instruction in 'decode' is at index 0.
    #[allow(unused)]
    pipeline: [u32; 2],
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
            gpr_banked_fiq: [0; 5],
            gpr_banked_r13: [0; 5],
            gpr_banked_r14: [0; 5],
            spsr: ProgramStatusRegister::new(),
            cpsr: ProgramStatusRegister::new(),
            // Starts filled with 0, which encodes a useless instruction
            // (but not the canonical no-op).
            pipeline: [0; 2],
        }
    }
}

impl Gba {
    /// Do a single CPU emulation step (not necessarily a single clock cycle).
    pub(crate) fn cpu_step(&mut self) {
        // Pump the pipeline.
        let opcode = self.cpu.pipeline[0];
        self.cpu.pipeline[0] = self.cpu.pipeline[1];
        eprintln!("cpu: PC={:08x}, opcode={:08x}", self.cpu.pc, opcode);

        match self.cpu.cpsr.execution_state {
            CpuExecutionState::Thumb => {
                // TODO: use correct memory fetch ordering
                self.cpu.pipeline[1] =
                    self.cpu_load16(self.cpu.pc, MemoryAccessType::Sequential) as u32;

                // TODO execute `opcode`.

                // Advance program counter.
                self.cpu.pc += 2;
            }
            CpuExecutionState::Arm => {
                // TODO: use correct memory fetch ordering
                self.cpu.pipeline[1] = self.cpu_load32(self.cpu.pc, MemoryAccessType::Sequential);

                match self.cpu_execute_arm(opcode) {
                    InstructionResult::Normal => {
                        // Advance program counter.
                        self.cpu.pc += 4;
                    }
                    InstructionResult::Branch(pc) => {
                        self.cpu_jump(pc);
                    }
                }
            }
        }
    }

    /// Jump to the given address (and flush the pipeline).
    fn cpu_jump(&mut self, pc: u32) {
        self.cpu.pipeline[0] = self.cpu_load32(pc, MemoryAccessType::NonSequential);
        self.cpu.pipeline[1] = self.cpu_load32(pc + 4, MemoryAccessType::Sequential);
        self.cpu.pc = pc;
    }

    /// Set a register.
    fn cpu_reg_set(&mut self, register: usize, value: u32) {
        assert!(register <= 14);
        match self.cpu.cpsr.mode {
            CpuMode::User | CpuMode::System => self.cpu.gpr[register] = value,
            _ if (register == 13) => self.cpu.gpr_banked_r13[register] = value,
            _ if (register == 14) => self.cpu.gpr_banked_r14[register] = value,
            CpuMode::Fiq if register >= 8 => self.cpu.gpr_banked_fiq[register - 8] = value,
            _ => self.cpu.gpr[register] = value,
        }
    }

    /// Get a register.
    fn cpu_reg_get(&mut self, register: usize) -> u32 {
        assert!(register <= 14);
        match self.cpu.cpsr.mode {
            CpuMode::User | CpuMode::System => self.cpu.gpr[register],
            _ if (register == 13) => self.cpu.gpr_banked_r13[register],
            _ if (register == 14) => self.cpu.gpr_banked_r14[register],
            CpuMode::Fiq if register >= 8 => self.cpu.gpr_banked_fiq[register - 8],
            _ => self.cpu.gpr[register],
        }
    }
}
