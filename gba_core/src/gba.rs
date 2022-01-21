use std::ops::DerefMut;

use crate::{
    cartridge::BackupType, interrupt::InterruptManager, io::CpuPowerState, Apu, BackupFile, Bus,
    Cartridge, Cpu, Dma, Event, Io, KeypadState, Ppu, Rom, Scheduler, TimerManager,
};

use serde::{Deserialize, Serialize};

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;

/// Game Boy Advance Emulator
#[derive(Serialize, Deserialize)]
pub struct Gba {
    /// The cartridge ROM.
    #[serde(skip)]
    pub(crate) cart_rom: Rom,
    /// The 16 KiB BIOS ROM.
    #[serde(skip)]
    pub(crate) bios_rom: Box<[u8]>,
    /// The cartridge backup file.
    #[serde(skip)]
    pub(crate) cart_backup_file: Option<Box<dyn BackupFile>>,

    /// CPU state.
    pub(crate) cpu: Cpu,

    /// Memory bus state.
    pub(crate) bus: Bus,

    /// Scheduler state: controls when events fire.
    pub(crate) scheduler: Scheduler,

    /// Memory mapped I/O state.
    pub(crate) io: Io,

    /// PPU state.
    pub(crate) ppu: Ppu,

    /// APU (sound) state.
    pub(crate) apu: Apu,

    /// Interrupt manager state.
    pub(crate) interrupt: InterruptManager,

    /// DMA controller state.
    pub(crate) dma: Dma,

    /// Timer state.
    pub(crate) timer: TimerManager,

    /// The cartridge.
    pub(crate) cartridge: Cartridge,

    /// On-board ("external") work RAM.
    pub(crate) ewram: Box<[u8]>,

    /// On-chip ("internal") work RAM.
    pub(crate) iwram: Box<[u8]>,

    /// How much we overshot the last emulate_cycles request by.
    last_emulation_overshoot: usize,

    /// Current keypad state.
    pub(crate) keypad_state: KeypadState,

    /// Whether we should generate audio and video.
    ///
    /// This may be set to false during fast forwarding.
    pub(crate) should_render: bool,
}

/// Builder struct for [`Gba`].
pub struct GbaBuilder {
    bios_rom: Box<[u8]>,
    cart_rom: Rom,

    /// Whether we should skip the BIOS boot animation.
    skip_bios: bool,

    /// The backing storage for the cartridge backup.
    backup_file: Option<Box<dyn BackupFile>>,

    /// Backup type (or None for autodetection).
    backup_type: Option<BackupType>,
}

impl Gba {
    /// Create a new GBA emulator builder.
    pub fn builder(bios_rom: Box<[u8]>, cart_rom: Rom) -> GbaBuilder {
        GbaBuilder {
            bios_rom,
            cart_rom,
            skip_bios: false,
            backup_file: None,
            backup_type: None,
        }
    }

    /// Create a new GBA emulator from the builder.
    fn build(builder: GbaBuilder) -> Gba {
        let cartridge = Cartridge::new(&builder.cart_rom, builder.backup_type);
        let mut gba = Gba {
            cart_rom: builder.cart_rom,
            bios_rom: builder.bios_rom,
            cart_backup_file: builder.backup_file,

            cpu: Cpu::new(),
            bus: Bus::new(),
            scheduler: Scheduler::new(),
            io: Io::new(),
            ppu: Ppu::new(),
            apu: Apu::new(),
            interrupt: InterruptManager::new(),
            dma: Dma::new(),
            timer: TimerManager::new(),
            cartridge,
            ewram: Box::new([0; 256 * 1024]),
            iwram: Box::new([0; 32 * 1024]),
            last_emulation_overshoot: 0,
            keypad_state: KeypadState::default(),
            should_render: false,
        };
        gba.ppu_init();
        gba.apu_init();

        // Load the backup file.
        if let Some(backup_file) = gba.cart_backup_file.as_mut() {
            gba.cartridge.backup_buffer.load(backup_file.deref_mut());
        }

        if builder.skip_bios {
            gba.cpu.skip_bios();
            gba.ppu.skip_bios();
        }

        gba
    }

    /// Run the emulator for at least the given number of cycles.
    /// Returns the number of cycles actually ran for.
    fn run(&mut self, cycles: usize) -> usize {
        let start_time = self.scheduler.timestamp();
        self.scheduler.push_event(Event::StopRunning, cycles);

        'outer: loop {
            while self.scheduler.timestamp() < self.scheduler.peek_deadline().unwrap() {
                let cpu_active = self.io.power_state == CpuPowerState::Normal;
                let dma_active = self.dma_active();

                match (cpu_active, dma_active) {
                    (_, true) => {
                        // DMA is active and runs while CPU is suspended.
                        self.dma_step();
                    }
                    (true, false) => {
                        // Check for IRQ.
                        if self.interrupt_pending() {
                            self.cpu_irq();
                        }

                        self.cpu_step();
                    }
                    (false, false) => {
                        // CPU is in halt state and no DMA is active. Skip to next interrupt.
                        if self.interrupt_pending() {
                            self.io.power_state = CpuPowerState::Normal;
                        } else {
                            self.scheduler.skip_to_next_event();
                            break;
                        }
                    }
                }
            }

            // Handle any events.
            while let Some((event, lateness)) = self.scheduler.pop_event() {
                match event {
                    Event::StopRunning => break 'outer,
                    Event::Ppu(ppu) => self.ppu_on_event(ppu, lateness),
                    // TODO maybe handle lateness?
                    Event::DmaActivate(channel) => self.dma_activate_channel(channel as usize),
                    Event::TimerUpdate => self.timer_handle_event(),
                    Event::AudioSample => self.apu_on_sample_event(lateness),
                    Event::AudioSequencerTick => self.apu_on_sequencer_event(lateness),
                }
            }
        }

        let end_time = self.scheduler.timestamp();
        end_time - start_time
    }

    /// Emulate a frame.
    pub fn emulate_frame(&mut self, should_render: bool) {
        self.should_render = should_render;
        const FRAME_CYCLES: usize = (240 + 68) * (160 + 68) * 4;
        self.emulate_cycles(FRAME_CYCLES);
        self.should_render = true;
    }

    /// Emulate for the given number of cycles.
    ///
    /// Starts by clearing the audio sample buffer.
    pub fn emulate_cycles(&mut self, cycles: usize) {
        self.apu_buffer_clear();

        if cycles <= self.last_emulation_overshoot {
            self.last_emulation_overshoot -= cycles;
        } else {
            let run_cycles = cycles - self.last_emulation_overshoot;
            let actually_ran = self.run(run_cycles);
            self.last_emulation_overshoot = actually_ran - run_cycles;
        }

        // Persist the backup buffer (if it's dirty).
        if let Some(backup_file) = self.cart_backup_file.as_mut() {
            self.cartridge.backup_buffer.save(backup_file.deref_mut());
        }
    }

    /// Get the frame buffer.
    /// (240 * 160) pixels, each pixel in ARGB format, row major.
    pub fn framebuffer(&self) -> &[u32] {
        &self.ppu.framebuffer
    }

    /// Get the audio samples created during the last frame.
    /// This is a sequence of samples, interleaving the left and right channels.
    pub fn audio_buffer(&self) -> &[i16] {
        self.apu_buffer()
    }

    /// Serialize the current state of the emulator.
    ///
    /// This does not modify the cartridge or BIOS ROMs.
    pub fn save_state(&self) -> Vec<u8> {
        bincode::serialize(&self).expect("Failed to serialize state")
    }

    pub fn load_state(&mut self, data: &[u8]) {
        let mut new_gba: Gba = bincode::deserialize(data).expect("Failed to deserialize state");

        // Swap the whole state, then replace the parts we didn't deserialize.
        use std::mem::swap;
        swap(self, &mut new_gba);
        swap(&mut self.cart_rom, &mut new_gba.cart_rom);
        swap(&mut self.bios_rom, &mut new_gba.bios_rom);
        swap(&mut self.cart_backup_file, &mut new_gba.cart_backup_file);
    }
}

impl GbaBuilder {
    /// Set whether the BIOS boot animation should be skipped.
    pub fn skip_bios(mut self, should_skip: bool) -> Self {
        self.skip_bios = should_skip;
        self
    }

    /// Set the backup file.
    pub fn backup_file(mut self, backup_file: Box<dyn BackupFile>) -> Self {
        self.backup_file = Some(backup_file);
        self
    }

    /// Set the backup type. Overrides autodetection.
    pub fn backup_type(mut self, backup_type: BackupType) -> Self {
        self.backup_type = Some(backup_type);
        self
    }

    /// Build the GBA emulator with the current configuration.
    pub fn build(self) -> Gba {
        Gba::build(self)
    }
}
