const WIDTH: usize = 240;
const HEIGHT: usize = 160;

use gba_core::Gba;
use minifb::{Key, Window, WindowOptions};

fn make_gba() -> Gba {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("Usage: gba <path to rom>");
        std::process::exit(1);
    }

    let bios_path = "roms/bios.bin";
    let bios = std::fs::read(bios_path).expect("failed to read bios");
    assert_eq!(bios.len(), 16 * 1024);

    let rom_path = &args[1];
    let rom_data = std::fs::read(rom_path).expect("failed to read ROM");
    let rom = gba_core::Rom::new(&rom_data);
    println!("Loaded {:?}", rom);

    let mut gba = gba_core::Gba::new(bios.into(), rom);
    gba.skip_bios();
    gba
}

fn main() {
    // Create the gba.
    let mut gba = make_gba();

    // Create the window.
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let window_options = WindowOptions {
        scale: minifb::Scale::X2,
        scale_mode: minifb::ScaleMode::Stretch,
        ..WindowOptions::default()
    };
    let mut window =
        Window::new("GBA", WIDTH, HEIGHT, window_options).expect("Failed to create window.");
    // Limit to ~60 FPS.
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    loop {
        if !window.is_open() || window.is_key_down(Key::Escape) {
            // User wants to exit.
            break;
        }

        // TODO: get input.

        // TODO: run emulator for a frame.

        // TODO: update buffer with data from emulator.
        for i in buffer.iter_mut() {
            *i = 0x00FF7733; // write something more funny here!
        }
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }

    gba.hack_run();
}
