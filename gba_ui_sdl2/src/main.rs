use std::{
    fs,
    time::{Duration, Instant},
};

use gba_core::{Gba, KeypadState, AUDIO_CHANNELS, AUDIO_SAMPLE_RATE};

use sdl2::keyboard::{Keycode, Mod, Scancode};
use sdl2::pixels::Color;

const WIDTH: u32 = gba_core::WIDTH as u32;
const HEIGHT: u32 = gba_core::HEIGHT as u32;
const SCALE: u32 = 2;

fn get_keypad_state(event_pump: &sdl2::EventPump) -> KeypadState {
    let mut keypad = KeypadState::default();
    let keyboard_state = event_pump.keyboard_state();
    let keys = keyboard_state
        .pressed_scancodes()
        .filter_map(Keycode::from_scancode);
    for key in keys {
        match key {
            Keycode::Z => keypad.a = true,
            Keycode::X => keypad.b = true,
            Keycode::RShift => keypad.select = true,
            Keycode::Return => keypad.start = true,
            Keycode::Up => keypad.up = true,
            Keycode::Down => keypad.down = true,
            Keycode::Left => keypad.left = true,
            Keycode::Right => keypad.right = true,
            Keycode::A => keypad.l = true,
            Keycode::S => keypad.r = true,
            _ => {}
        }
    }
    keypad
}

fn run_emulator(mut gba: Gba) -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let audio_subsystem = sdl_context.audio()?;

    let window = video_subsystem
        .window("GBA", WIDTH * SCALE, HEIGHT * SCALE)
        .opengl()
        .position_centered()
        .allow_highdpi()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(sdl2::pixels::PixelFormatEnum::ARGB8888, WIDTH, HEIGHT)
        .map_err(|e| e.to_string())?;
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    let audio_spec_desired = sdl2::audio::AudioSpecDesired {
        freq: Some(gba_core::AUDIO_SAMPLE_RATE as i32),
        channels: Some(2),
        // Number of samples in the audio buffer. Must be a power of two.
        // We want this to be no more than ~0.5 frames, otherwise the audio drain
        // callback doesn't run frequently enough and we have to
        // either increase our buffer (causing latency) or deal with stutters.
        // This (256) with a sample rate of 32*1024 Hz is ~0.469 frames.
        samples: Some(256),
    };
    let audio_device = audio_subsystem.open_queue::<i16, _>(None, &audio_spec_desired)?;
    audio_device.resume();

    let mut frame_counter = 0;
    let mut frame_timer = Instant::now();
    let mut paused = false;
    let mut single_step = false;
    let mut was_paused = paused; // Was paused before focus lost.

    let mut event_pump = sdl_context.event_pump()?;
    let mut last_event: Option<sdl2::event::Event> = None;
    'running: loop {
        // Handle events.
        loop {
            // Allow for events we waited for previously.
            if last_event.is_none() {
                last_event = event_pump.poll_event();
                if last_event.is_none() {
                    break;
                }
            }
            match last_event.take().unwrap() {
                sdl2::event::Event::Quit { .. } => {
                    break 'running;
                }
                sdl2::event::Event::Window { win_event, .. } => match win_event {
                    sdl2::event::WindowEvent::FocusGained => {
                        paused = was_paused;
                    }
                    sdl2::event::WindowEvent::FocusLost => {
                        was_paused = paused;
                        paused = true;
                    }
                    _ => {}
                },
                sdl2::event::Event::KeyDown {
                    keycode: Some(code),
                    keymod,
                    ..
                } => {
                    let command = keymod == Mod::LGUIMOD || keymod == Mod::RGUIMOD;
                    match code {
                        Keycode::P if command => {
                            paused = !paused;
                        }
                        Keycode::N if command => {
                            paused = true;
                            single_step = true;
                        }
                        Keycode::Escape => {
                            break 'running;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        let keypad = get_keypad_state(&event_pump);
        gba.set_keypad_state(keypad);

        let fast_forward = event_pump
            .keyboard_state()
            .is_scancode_pressed(Scancode::Tab);
        if !paused || single_step {
            single_step = false;

            let speed = if fast_forward { 4 } else { 1 };
            for i in 0..speed {
                gba.emulate_frame(i == speed - 1);
                frame_counter += 1;
            }

            let buffer = gba.framebuffer();
            let buffer = unsafe { std::mem::transmute::<&[u32], &[u8]>(buffer) };
            texture
                .update(None, buffer, (WIDTH * 4) as usize)
                .map_err(|e| e.to_string())?;
            canvas.copy(&texture, None, None)?;
            canvas.present();

            // Each "sample frame" is a sample from (left, right) -- 4 bytes.
            let samples_queued =
                (audio_device.size() as usize) / (std::mem::size_of::<i16>() * AUDIO_CHANNELS);
            // let frames_queued = 60.0 * ((samples_queued as f64) / (AUDIO_SAMPLE_RATE as f64));
            // println!("audio queued: {:>7} samples  =  {:>7}frames", samples_queued, frames_queued);
            // Target maximum of 2 frames of samples in the buffer.
            let samples_max = 2 * AUDIO_SAMPLE_RATE / 60;
            if samples_queued < samples_max {
                let buffer = gba.audio_buffer();
                let to_add = buffer
                    .len()
                    .min((samples_max - samples_queued) * AUDIO_CHANNELS);
                audio_device.queue(&buffer[..to_add]);
            }
        } else {
            last_event = Some(event_pump.wait_event());
        }

        // FPS display
        if Instant::now() - frame_timer > Duration::from_secs(1) {
            canvas
                .window_mut()
                .set_title(&format!("GBA - FPS: {}", frame_counter))
                .map_err(|e| e.to_string())?;
            frame_counter = 0;
            frame_timer = Instant::now();
        }
    }

    Ok(())
}

fn main() {
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

    let rom_base_path = if rom_path.ends_with(".gba") {
        &rom_path[..(rom_path.len() - 4)]
    } else {
        rom_path
    };
    let backup_path = format!("{}.sav", rom_base_path);
    println!("Using cartridge save path {}", backup_path);
    let backup_file = gba_core::util::make_backup_file(backup_path);

    let gba = gba_core::Gba::builder(bios.into(), rom)
        .skip_bios(true)
        .backup_file(backup_file)
        .build();

    run_emulator(gba).unwrap();
}
