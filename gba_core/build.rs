use std::io::Write;
use std::{fs::File, path::Path};

/// Return the handler for the given instruction base.
fn decode_arm_entry(_inst: u32) -> String {
    "arm_unimplemented".to_string()
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
