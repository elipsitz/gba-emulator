# GBA Emulator

## Acknowledgements

Everybody in `#gba` on the [Emulation Development Discord](https://discord.gg/dkmJAes)!

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