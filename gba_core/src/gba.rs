use crate::{Cpu, Rom};

/// Game Boy Advance Emulator
pub struct Gba {
    /// CPU state.
    pub(crate) cpu: Cpu,

    /// CPU cycle counter.
    #[allow(unused)]
    pub(crate) cycles: usize,

    /// The 16 KiB BIOS ROM.
    pub(crate) bios_rom: Box<[u8]>,

    /// The cartridge ROM.
    pub(crate) cart_rom: Rom,

    /// On-board ("external") work RAM.
    pub(crate) ewram: [u8; 256 * 1024],

    /// On-chip ("internal") work RAM.
    pub(crate) iwram: [u8; 32 * 1024],
}

impl Gba {
    /// Create a new GBA emulator from the given BIOS and cartridge.
    pub fn new(bios_rom: Box<[u8]>, cart_rom: Rom) -> Gba {
        Gba {
            cpu: Cpu::new(),
            cycles: 0,
            bios_rom,
            cart_rom,
            ewram: [0; 256 * 1024],
            iwram: [0; 32 * 1024],
        }
    }

    pub fn skip_bios(&mut self) {
        self.cpu.skip_bios();
    }

    /// Temporary: run the CPU for a bunch of cycles.
    pub fn hack_run(&mut self) {
        for _ in 0..1000 {
            self.cpu_step();
        }
    }
}
