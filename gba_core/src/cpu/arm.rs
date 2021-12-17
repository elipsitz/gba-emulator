use super::{cond::Condition, InstructionResult};
use crate::Gba;

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
    panic!("Unknown ARM instruction: {:08x} / {:032b}", inst, inst);
}

// Include look-up table for instruction handlers.
include!(concat!(env!("OUT_DIR"), "/arm_table.rs"));

impl Gba {
    /// Get the program counter of the *currently executing instruction*.
    fn inst_arm_pc(&self) -> u32 {
        // Go back 2 instructions (because pipelining).
        self.cpu.pc.wrapping_sub(8)
    }

    /// Execute the given ARM instruction.
    pub(super) fn cpu_execute_arm(&mut self, inst: u32) -> InstructionResult {
        let condition: Condition = inst.bit_range(28..32).into();
        if !condition.evaluate(self) {
            return InstructionResult::Normal;
        }

        let key = (((inst >> 16) & 0xff0) | ((inst >> 4) & 0xf)) as usize;
        (ARM_HANDLERS[key])(self, inst)
    }

    fn exec_branch(&mut self, inst: u32) -> InstructionResult {
        // Current PC is actually PC + 8 due to pipeline.
        let offset = ((inst.bit_range(0..24) << 8) as i32) >> 6;
        let pc = ((self.cpu.pc as i32) + offset) as u32;

        let link = inst.bit(24);
        if link {
            // Branch and link.
            self.cpu_reg_set(14, self.inst_arm_pc() + 4);
        }

        InstructionResult::Jump(pc)
    }
}
