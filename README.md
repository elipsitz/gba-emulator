# Game Boy Advance Emulator

This is a Game Boy Advance (GBA) emulator, written in Rust. It's fairly accurate and efficient,
and can play all of the games I tested without any issues.

I'd previously written an NES emulator, [first in Go](https://github.com/elipsitz/nes), and then
[rewritten and improved in Rust](https://github.com/elipsitz/nes-rs/). For my next emulation
project, I wanted to emulate a system that I had a closer personal connection to.

## Features
* Good accuracy (enough to play games without any problems)
* Full audio support
* Cartridge saves and emulator save states
* RTC emulation (e.g. for Pokemon)

### Future Work
* GUI (perhaps using imgui)
* Higher quality audio resampling and syncing
* Maybe: link cable support
* Maybe: more accurate timing (e.g. cartridge prefetch buffer, DMA)
* Maybe: debugger

### Known Minor Inaccuracies
* DMA open bus isn't properly implemented
* Slight click when PSG audio channels change frequency
* Prefetch buffer is approximated as 1 cycle per access
* Mosaic on affine backgrounds is unsupported
* Tile indexing allows doing things hardware doesn't
  (e.g. accessing invalid tileblocks for the current mode)
* SOUNDBIAS sampling rate isn't implemented (it's fixed to 32 KHz)

## Usage

```
gba_emulator [OPTIONS] --bios-path <BIOS_PATH> <ROM_PATH>
```

You'll need to provide a GBA BIOS ROM. I've only tested with the official one, but 
others should work too.

I've developed and tested this emulator on macOS. Theoretically, it should work
on any platform SDL2 supports (including Windows and Linux).

### Controls
* `Z`: A button
* `X`: B button
* `A`: L button
* `S`: R button
* `Enter`: Start
* `Right Shift`: Select
* `Arrow keys`: D-Pad

There are also a few keyboard shortcuts to control the emulator itself:
* `Cmd-P`: Pause/Resume emulation
* `Cmd-N`: Step forward one frame
* `Cmd-S`: Save the save state
* `Cmd-L`: Load the save state
* `Tab`: Hold to fast-forward (4x speed)

Save states are saved to the same directory as the ROM, with the `.save_state` extension.
These are unique to the emulator.

Cartridge saves are saved to the same directory, with the `.sav` extension. These should
be transferrable between any emulator (or a physical cartridge).

## Building

You'll need a relatively recent version of Rust, as well as SDL2. Then, it's
as simple as running `cargo build --release`. Make sure to build in release mode: debug
is likely too slow to run games at full speed.

## Acknowledgements

This project wouldn't have been possible without a lot of resources from the 
emulation community. 

I'd also like to thank everybody in `#gba` on the [Emulation Development Discord](https://discord.gg/dkmJAes),
for answering questions and providing support.

### Resources
* ARM Architecture Reference Manual
* ARM7TDMI-S Technical Reference Manual
* [GBATEK](https://problemkaputt.de/gbatek.htm): The main GBA resource, explains everything
* [Rodrigo Copetti's Game Boy Advance Architecture](https://www.copetti.org/writings/consoles/game-boy-advance/):
  an interesting, detailed, and informative resource (and not just for GBA, either).
* [TONC](https://www.coranac.com/tonc/text/toc.htm): Excellent tutorial on GBA programming.
  Especially useful as a resource for understanding the PPU (along with the demos)
* [The Audio Advance](http://belogic.com/gba/): Detailed explanation of GBA audio (with demos).
* [Gameboy sound hardware](https://gbdev.gg8.se/wiki/articles/Gameboy_sound_hardware): detailed
  explanation of the original Gameboy's APU, which is part of the GBA's APU.
* [The mGBA Blog](https://mgba.io/)
* DenSinH's explanations of [Flash](https://dillonbeliveau.com/2020/06/05/GBA-FLASH.html)
  and [EEPROM](https://densinh.github.io/DenSinH/emulation/2021/02/01/gba-eeprom.html).

### Tests / Demo ROMS
* [jsmolka's CPU tests](https://github.com/jsmolka/gba-tests): The first ROMs I used to test 
  my CPU as I was implementing it. Very thorough!
* [armwrestler](https://github.com/destoer/armwrestler-gba-fixed): more CPU tests
* [FuzzARM](https://github.com/DenSinH/FuzzARM): randomly generated CPU tests
* [TONC Demos](https://www.coranac.com/projects/#tonc): Graphics demos that go along with the
  TONC guide.
* [The Audio Advance Demos](http://belogic.com/gba/): GBA audio demos
* [DenSinH's Flash and EEPROM Tests](https://github.com/DenSinH/GBARoms)
* [MichelOS's RTC Demo](https://github.com/michelhe/gba-playground/tree/master/rtc-demo)

### Emulators
* [mGBA](https://github.com/mgba-emu/mgba): I relied heavily on the debugging features when I was
  first working on the CPU, as well on the PPU inspection tools.
* [NanoBoyAdvance](https://github.com/nba-emu/NanoBoyAdvance): I used NanoBoyAdvance as a reference
  for certain details of the GBA's behavior, especially when GBATEK was too terse.
* [RustBoyAdvance-NG](https://github.com/michelhe/rustboyadvance-ng/): I used this as inspiration
  for some Rust-specific design decisions (especially the instruction decoder table).