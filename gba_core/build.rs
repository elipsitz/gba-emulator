use std::io::Write;
use std::{fs::File, path::Path};

use bit::BitIndex;

/// Return the ARM handler for the given instruction base.
fn decode_arm_entry(inst: u32) -> String {
    match inst.bit_range(25..28) {
        0b000 | 0b001 if !(inst.bit_range(23..25) == 0b10 && !inst.bit(20)) => {
            // ALU data processing instructions.
            // opcode = 21..25
            // imm = 25
            // set = 20
            // shift type (if !imm) = 5..7
            // regshift = 4
            format!(
                "arm_exec_alu::<{OPCODE}, {IMM}, {SETCOND}, {SHIFT_TYPE}, {REGSHIFT}>",
                OPCODE = inst.bit_range(21..25),
                IMM = inst.bit(25),
                SETCOND = inst.bit(20),
                SHIFT_TYPE = inst.bit_range(5..7),
                REGSHIFT = inst.bit(4) && inst.bit(25),
            )
        }
        0b000 if (inst.bit_range(23..25) == 0b10 && !inst.bit(20)) => {
            // Miscellaneous functions.
            match (inst.bit_range(4..8), inst.bit_range(21..23)) {
                (0b0000, 0b00 | 0b10) => format!("arm_exec_mrs::<{R}>", R = inst.bit(22)),
                (0b0000, 0b01 | 0b11) => format!("arm_exec_msr::<{R}, false>", R = inst.bit(22)),
                (0b0001, 0b01) => format!("arm_exec_branch_exchange"),
                _ => "arm_unimplemented".to_string(),
            }
        }
        0b001 if (inst.bit_range(20..25) & 0b11011) == 0b10010 => {
            // Move immediate to status register.
            format!("arm_exec_msr::<{R}, true>", R = inst.bit(22))
        }
        0b101 => {
            // Branch, Branch-and-link.
            format!("arm_exec_branch::<{LINK}>", LINK = inst.bit(24))
        }
        0b010 | 0b011 => {
            // Load and Store word or unsigned byte.
            format!(
                "arm_exec_ldr_str_word_byte::<{IMMEDIATE}, {PREINDEX}, {UP}, {BYTE}, {WRITEBACK}, {LOAD}, {SHIFT_TYPE}>",
                IMMEDIATE = inst.bit(25),
                PREINDEX = inst.bit(24),
                UP = inst.bit(23),
                BYTE = inst.bit(22),
                WRITEBACK = inst.bit(21),
                LOAD = inst.bit(20),
                SHIFT_TYPE = inst.bit_range(5..7),
            )
        }
        0b100 => {
            // Load/store multiple.
            format!(
                "arm_exec_ldm_stm::<{PREINDEX}, {UP}, {S}, {WRITEBACK}, {LOAD}>",
                PREINDEX = inst.bit(24),
                UP = inst.bit(23),
                S = inst.bit(22),
                WRITEBACK = inst.bit(21),
                LOAD = inst.bit(20),
            )
        }
        _ => "arm_unimplemented".to_string(),
    }
}

/// Return the Thumb handler for the given instruction base.
fn decode_thumb_entry(inst: u16) -> String {
    "thumb_unimplemented".to_string()
}

fn generate_arm_table(file: &mut File) -> std::io::Result<()> {
    writeln!(file, "static ARM_HANDLERS: [ArmHandler; 4096] = [")?;
    for key in 0u32..4096 {
        // Rehydrate instruction base:
        let inst = ((key & 0xff0) << 16) | ((key & 0xf) << 4);
        let handler = decode_arm_entry(inst);
        writeln!(file, "    {},", handler)?;
    }
    writeln!(file, "];")?;

    Ok(())
}

fn generate_thumb_table(file: &mut File) -> std::io::Result<()> {
    writeln!(file, "static THUMB_HANDLERS: [ThumbHandler; 1024] = [")?;
    for key in 0u16..1024 {
        // Rehydrate instruction base:
        let inst = key << 6;
        let handler = decode_thumb_entry(inst);
        writeln!(file, "    {},", handler)?;
    }
    writeln!(file, "];")?;

    Ok(())
}

fn main() {
    let output_dir = std::env::var_os("OUT_DIR").unwrap();

    let arm_path = Path::new(&output_dir).join("arm_table.rs");
    let mut arm_file = File::create(&arm_path).unwrap();
    generate_arm_table(&mut arm_file).unwrap();

    let thumb_path = Path::new(&output_dir).join("thumb_table.rs");
    let mut thumb_file = File::create(&thumb_path).unwrap();
    generate_thumb_table(&mut thumb_file).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
