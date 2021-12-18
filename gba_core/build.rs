use std::io::Write;
use std::{fs::File, path::Path};

use bit::BitIndex;

/// Return the handler for the given instruction base.
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
        0b101 => {
            // Branch, Branch-and-link.
            format!("arm_exec_branch::<{LINK}>", LINK = inst.bit(24))
        }
        _ => "arm_unimplemented".to_string(),
    }
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

fn main() {
    let output_dir = std::env::var_os("OUT_DIR").unwrap();

    let arm_path = Path::new(&output_dir).join("arm_table.rs");
    let mut arm_file = File::create(&arm_path).unwrap();
    generate_arm_table(&mut arm_file).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
