use crate::{Bus, Cpu, Event, Rom, Scheduler};

/// Game Boy Advance Emulator
pub struct Gba {
    /// CPU state.
    pub(crate) cpu: Cpu,

    /// Memory bus state.
    pub(crate) bus: Bus,

    /// Scheduler state: controls when events fire.
    pub(crate) scheduler: Scheduler,

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

    /// How much we overshot the last frame by.
    last_frame_overshoot: usize,
}

impl Gba {
    /// Create a new GBA emulator from the given BIOS and cartridge.
    pub fn new(bios_rom: Box<[u8]>, cart_rom: Rom) -> Gba {
        Gba {
            cpu: Cpu::new(),
            bus: Bus::new(),
            scheduler: Scheduler::new(),
            cycles: 0,
            bios_rom,
            cart_rom,
            ewram: [0; 256 * 1024],
            iwram: [0; 32 * 1024],
            last_frame_overshoot: 0,
        }
    }

    pub fn skip_bios(&mut self) {
        self.cpu.skip_bios();
    }

    /// Run the emulator for at least the given number of cycles.
    /// Returns the number of cycles actually ran for.
    fn run(&mut self, cycles: usize) -> usize {
        let start_time = self.scheduler.timestamp();
        self.scheduler.push_event(Event::StopRunning, cycles);

        'outer: loop {
            while self.scheduler.timestamp() < self.scheduler.peek_deadline().unwrap() {
                self.cpu_step();
            }

            // Handle any events.
            while let Some((event, _lateness)) = self.scheduler.pop_event() {
                match event {
                    Event::StopRunning => break 'outer,
                }
            }
        }

        let end_time = self.scheduler.timestamp();
        end_time - start_time
    }

    /// Emulate a frame.
    pub fn emulate_frame(&mut self) {
        let frame_cycles = (240 + 68) * (160 * 68) * 4;
        let run_cycles = frame_cycles - self.last_frame_overshoot;
        let actually_ran = self.run(run_cycles);
        self.last_frame_overshoot = actually_ran - run_cycles;
    }
}
