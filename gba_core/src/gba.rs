use crate::{
    interrupt::InterruptManager, io::CpuPowerState, Bus, Cpu, Event, Io, KeypadState, Ppu, Rom,
    Scheduler,
};

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;

/// Game Boy Advance Emulator
pub struct Gba {
    /// CPU state.
    pub(crate) cpu: Cpu,

    /// Memory bus state.
    pub(crate) bus: Bus,

    /// Scheduler state: controls when events fire.
    pub(crate) scheduler: Scheduler,

    /// Memory mapped I/O state.
    #[allow(unused)]
    pub(crate) io: Io,

    /// PPU state.
    pub(crate) ppu: Ppu,

    /// Interrupt manager state.
    pub(crate) interrupt: InterruptManager,

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

    /// Current keypad state.
    pub(crate) keypad_state: KeypadState,
}

impl Gba {
    /// Create a new GBA emulator from the given BIOS and cartridge.
    pub fn new(bios_rom: Box<[u8]>, cart_rom: Rom) -> Gba {
        let mut gba = Gba {
            cpu: Cpu::new(),
            bus: Bus::new(),
            scheduler: Scheduler::new(),
            io: Io::new(),
            ppu: Ppu::new(),
            interrupt: InterruptManager::new(),
            bios_rom,
            cart_rom,
            ewram: [0; 256 * 1024],
            iwram: [0; 32 * 1024],
            last_frame_overshoot: 0,
            keypad_state: KeypadState::default(),
        };
        gba.ppu_init();
        gba
    }

    pub fn skip_bios(&mut self) {
        self.cpu.skip_bios();
        self.ppu.skip_bios();
    }

    /// Run the emulator for at least the given number of cycles.
    /// Returns the number of cycles actually ran for.
    fn run(&mut self, cycles: usize) -> usize {
        let start_time = self.scheduler.timestamp();
        self.scheduler.push_event(Event::StopRunning, cycles);

        'outer: loop {
            while self.scheduler.timestamp() < self.scheduler.peek_deadline().unwrap() {
                if self.io.power_state == CpuPowerState::Normal {
                    // Check for IRQ.
                    if self.interrupt_pending() {
                        self.cpu_irq();
                    }

                    self.cpu_step();
                } else {
                    // CPU is in halt state. Skip to the next interrupt.
                    if self.interrupt_pending() {
                        self.io.power_state = CpuPowerState::Normal;
                    } else {
                        self.scheduler.skip_to_next_event();
                        break;
                    }
                }
            }

            // Handle any events.
            while let Some((event, lateness)) = self.scheduler.pop_event() {
                match event {
                    Event::StopRunning => break 'outer,
                    Event::Ppu(ppu) => self.ppu_on_event(ppu, lateness),
                }
            }
        }

        let end_time = self.scheduler.timestamp();
        end_time - start_time
    }

    /// Emulate a frame.
    pub fn emulate_frame(&mut self) {
        let frame_cycles = (240 + 68) * (160 + 68) * 4;
        let run_cycles = frame_cycles - self.last_frame_overshoot;
        let actually_ran = self.run(run_cycles);
        self.last_frame_overshoot = actually_ran - run_cycles;
    }

    /// Get the frame buffer.
    /// (240 * 160) pixels, each pixel in ARGB format, row major.
    pub fn framebuffer(&self) -> &[u32] {
        &self.ppu.framebuffer
    }
}
