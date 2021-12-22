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

        // Run emulator for a frame.
        gba.emulate_frame();

        // Update window with the framebuffer.
        let framebuffer = gba.framebuffer();
        window
            .update_with_buffer(framebuffer, WIDTH, HEIGHT)
            .unwrap();
    }
}
