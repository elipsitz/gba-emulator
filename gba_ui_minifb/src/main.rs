const WIDTH: usize = 240;
const HEIGHT: usize = 160;

use gba_core::{BackupFile, Gba, KeypadState};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::time::{Duration, Instant};

const TARGET_FPS: Duration = Duration::from_nanos(1_000_000_000 / 60);

fn make_backup_file(path: String) -> Box<dyn BackupFile> {
    /// File-backed BackupFile.
    struct DiskBackup {
        path: String,
        file: Option<fs::File>,
        buffer: Vec<u8>,
    }

    impl BackupFile for DiskBackup {
        fn initialize(&mut self, size: usize) {
            self.buffer.resize(size, 0);
            let mut file = fs::File::options()
                .create(true)
                .read(true)
                .write(true)
                .open(&self.path)
                .expect("Failed to open backup file");
            let file_size = file.seek(SeekFrom::End(0)).unwrap() as usize;
            let to_read = file_size.min(size);
            file.seek(SeekFrom::Start(0)).unwrap();
            file.read_exact(&mut self.buffer[..to_read]).unwrap();
            file.seek(SeekFrom::Start(0)).unwrap();
            file.write_all(&self.buffer).unwrap();
            self.file = Some(file);
        }

        fn read(&mut self, offset: usize, buffer: &mut [u8]) {
            buffer.copy_from_slice(&self.buffer[offset..(offset + buffer.len())]);
        }

        fn write(&mut self, offset: usize, data: &[u8]) {
            self.buffer[offset..(offset + data.len())].copy_from_slice(&data);
            if let Some(file) = &mut self.file {
                file.seek(SeekFrom::Start(offset as u64)).unwrap();
                file.write_all(data).unwrap();
            }
        }
    }

    Box::new(DiskBackup {
        path,
        file: None,
        buffer: Vec::new(),
    })
}

fn make_gba() -> Gba {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("Usage: gba <path to rom>");
        std::process::exit(1);
    }

    let bios_path = "roms/bios.bin";
    let bios = fs::read(bios_path).expect("failed to read bios");
    assert_eq!(bios.len(), 16 * 1024);

    let rom_path = &args[1];
    let rom_data = fs::read(rom_path).expect("failed to read ROM");
    let rom = gba_core::Rom::new(&rom_data);
    println!("Loaded {:?}", rom);

    let backup_path = format!("{}.sav", rom_path);
    let backup_file = make_backup_file(backup_path);

    gba_core::Gba::builder(bios.into(), rom)
        .skip_bios(true)
        .backup_file(backup_file)
        .build()
}

fn main() {
    // Create the gba.
    let mut gba = make_gba();

    // Create the window.
    let window_options = WindowOptions {
        scale: minifb::Scale::X2,
        scale_mode: minifb::ScaleMode::Stretch,
        topmost: true,
        ..WindowOptions::default()
    };
    let mut window =
        Window::new("GBA", WIDTH, HEIGHT, window_options).expect("Failed to create window.");
    // Limit to ~60 FPS.
    window.limit_update_rate(Some(TARGET_FPS));

    let mut paused = false;
    let mut single_step = false;
    let mut cap_framerate = true;

    let mut frame_counter = 0;
    let mut last_fps_update = Instant::now();
    loop {
        if !window.is_open() || window.is_key_down(Key::Escape) {
            // User wants to exit.
            println!("Exiting.");
            break;
        }
        if window.is_key_pressed(Key::Space, KeyRepeat::No) {
            paused = !paused;
            if paused {
                println!("Paused.");
            } else {
                println!("Unpaused.");
            }
        }
        if window.is_key_pressed(Key::Tab, KeyRepeat::Yes) {
            if !paused {
                println!("Paused.");
            }
            paused = true;
            single_step = true;
        }
        if window.is_key_pressed(Key::Backslash, KeyRepeat::No) {
            cap_framerate = !cap_framerate;
            if cap_framerate {
                println!("Capped framerate.");
                window.limit_update_rate(Some(TARGET_FPS));
            } else {
                println!("Uncapped framerate.");
                window.limit_update_rate(None);
            }
        }

        // Get keypad input.
        let mut keypad = KeypadState::default();
        keypad.a = window.is_key_down(Key::Z);
        keypad.b = window.is_key_down(Key::X);
        keypad.select = window.is_key_down(Key::RightShift);
        keypad.start = window.is_key_down(Key::Enter);
        keypad.right = window.is_key_down(Key::Right);
        keypad.left = window.is_key_down(Key::Left);
        keypad.up = window.is_key_down(Key::Up);
        keypad.down = window.is_key_down(Key::Down);
        keypad.r = window.is_key_down(Key::S);
        keypad.l = window.is_key_down(Key::A);
        gba.set_keypad_state(keypad);

        if !paused || single_step {
            single_step = false;

            // Run emulator for a frame.
            gba.emulate_frame();
            frame_counter += 1;

            // Update window with the framebuffer.
            let framebuffer = gba.framebuffer();
            window
                .update_with_buffer(framebuffer, WIDTH, HEIGHT)
                .unwrap();
        } else {
            window.update();
        }

        // Update FPS counter.
        let elapsed = Instant::now() - last_fps_update;
        if elapsed >= Duration::from_secs(1) {
            window.set_title(&format!("GBA (FPS: {})", frame_counter));
            frame_counter = 0;
            last_fps_update = Instant::now();
        }
    }
}
