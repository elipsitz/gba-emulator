use std::io::Write;
use std::{fs::File, path::Path};

use bit::BitIndex;

/// Return the ARM handler for the given instruction base.
fn decode_arm_entry(inst: u32) -> String {
    match inst.bit_range(25..28) {
        0b000 if inst.bit(4) && inst.bit(7) => {
            // Multiply and extra loads/stores.
            if inst.bit_range(4..8) == 0b1001 && inst.bit_range(23..28) == 0b00000 {
                // Multiply (accumulate)
                format!(
                    "arm_exec_mul::<{ACCUMULATE}, {SET_FLAGS}>",
                    ACCUMULATE = inst.bit(21),
                    SET_FLAGS = inst.bit(20),
                )
            } else if inst.bit_range(4..8) == 0b1001 && inst.bit_range(23..28) == 0b00001 {
                // Multiply (accumulate) long
                format!(
                    "arm_exec_mul_long::<{SIGNED}, {ACCUMULATE}, {SET_FLAGS}>",
                    SIGNED = inst.bit(22),
                    ACCUMULATE = inst.bit(21),
                    SET_FLAGS = inst.bit(20),
                )
            } else if inst.bit_range(4..8) == 0b1001 && inst.bit_range(23..28) == 0b00010 {
                // Swap / swap byte.
                format!("arm_exec_swap::<{BYTE}>", BYTE = inst.bit(22))
            } else {
                format!(
                    "arm_exec_ld_st_halfword_byte::<{PREINDEX}, {UP}, {IMMEDIATE}, {WRITEBACK}, {LOAD}, {SIGNED}, {HALFWORD}>",
                    PREINDEX = inst.bit(24),
                    UP = inst.bit(23),
                    IMMEDIATE = inst.bit(22),
                    WRITEBACK = inst.bit(21),
                    LOAD = inst.bit(20),
                    SIGNED = inst.bit(6),
                    HALFWORD = inst.bit(5),
                )
            }
        }
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
                REGSHIFT = inst.bit(4) && !inst.bit(25),
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
        0b111 if inst.bit(24) => {
            // Software interrupt.
            "arm_exec_swi".to_string()
        }
        _ => "arm_unimplemented".to_string(),
    }
}

/// Return the Thumb handler for the given instruction base.
fn decode_thumb_entry(inst: u16) -> String {
    if u16_matches(inst, "000 ** ***** *** ***") {
        let opcode = inst.bit_range(11..13);
        if opcode != 0b11 {
            // THUMB.1: shift by immediate
            format!("thumb_exec_shift_imm::<{OPCODE}>", OPCODE = opcode)
        } else {
            // THUMB.2: add / subtract
            format!(
                "thumb_exec_add_sub::<{IMM}, {SUB}>",
                IMM = inst.bit(10),
                SUB = inst.bit(9),
            )
        }
    } else if u16_matches(inst, "001 ** *** ********") {
        // THUMB.3: move/compare/add/subtract immediate
        format!(
            "thumb_exec_alu_immediate::<{OPCODE}, {REG_D}>",
            OPCODE = inst.bit_range(11..13),
            REG_D = inst.bit_range(8..11),
        )
    } else if u16_matches(inst, "010000 **** *** ***") {
        // THUMB.4: data-processing register
        format!(
            "thumb_exec_alu_register::<{OPCODE}>",
            OPCODE = inst.bit_range(6..10)
        )
    } else if u16_matches(inst, "010001 ** * * *** ***") {
        // THUMB.5: Hi register operations/branch exchange.
        let opcode = inst.bit_range(8..10);
        if opcode == 0b11 {
            format!("thumb_exec_bx::<{MSB_REG_S}>", MSB_REG_S = inst.bit(6),)
        } else {
            format!(
                "thumb_exec_hireg::<{OPCODE}, {MSB_REG_D}, {MSB_REG_S}>",
                OPCODE = inst.bit_range(8..10),
                MSB_REG_D = inst.bit(7),
                MSB_REG_S = inst.bit(6),
            )
        }
    } else if u16_matches(inst, "01001 *** ********") {
        // THUMB.6: load PC-relative
        "thumb_exec_load_pc_relative".to_string()
    } else if u16_matches(inst, "0101 *** *** *** ***") {
        // THUMB.7: load/store with register offset
        // THUMB.8: load/store sign-extended byte/halfword
        format!(
            "thumb_exec_ldr_str_reg_offset::<{OP}>",
            OP = inst.bit_range(9..12)
        )
    } else if u16_matches(inst, "011 * * ***** *** ***") {
        // THUMB.9 load/store with immediate offset
        format!(
            "thumb_exec_ldr_str_imm::<{BYTE}, {LOAD}>",
            BYTE = inst.bit(12),
            LOAD = inst.bit(11),
        )
    } else if u16_matches(inst, "1010 * *** ********") {
        // THUMB.12: get relative address
        format!("thumb_exec_address_calc::<{SP}>", SP = inst.bit(11),)
    } else if u16_matches(inst, "10110000 * *******") {
        // THUMB.13: add offset to stack pointer
        format!("thumb_exec_adjust_sp::<{SUB}>", SUB = inst.bit(7))
    } else if u16_matches(inst, "1101 **** ********") {
        let middle = inst.bit_range(8..12);
        match middle {
            // Undefined instruction.
            0b1110 => "thumb_unimplemented".to_string(),
            // THUMB.17: software interrupt
            0b1111 => "thumb_exec_swi".to_string(),
            // THUMB.16: conditional branch
            _ => format!("thumb_exec_branch_conditional::<{COND}>", COND = middle),
        }
    } else if u16_matches(inst, "11100 ***********") {
        // THUMB.18: branch
        "thumb_exec_branch".to_string()
    } else if u16_matches(inst, "1111 * ***********") {
        // THUMB.19: branch and link
        format!("thumb_exec_branch_link::<{SUFFIX}>", SUFFIX = inst.bit(11))
    } else {
        "thumb_unimplemented".to_string()
    }
}

/// Returns whether the given u16 matches the given bit pattern.
/// Bit pattern consists of a series of 0, 1, and *.
fn u16_matches(num: u16, pattern: &str) -> bool {
    let mut index = 15;
    for char in pattern.chars() {
        if char == '1' || char == '0' || char == '*' {
            let bit = num.bit(index);
            match char {
                '1' => {
                    if !bit {
                        return false;
                    }
                }
                '0' => {
                    if bit {
                        return false;
                    }
                }
                _ => {}
            }
            if index == 0 {
                return true;
            } else {
                index -= 1;
            }
        }
    }
    unreachable!("Got to the end of the pattern without hitting all bits");
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
