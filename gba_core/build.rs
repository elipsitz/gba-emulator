use std::io::Write;
use std::{fs::File, path::Path};

use bit::BitIndex;

/// Return the handler for the given instruction base.
fn decode_arm_entry(inst: u32) -> String {
    match inst.bit_range(25..28) {
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
