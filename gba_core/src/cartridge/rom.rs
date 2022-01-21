use super::BackupType;

/// A Game Boy Advance ROM.
pub struct Rom {
    /// The ROM data.
    pub(crate) data: Box<[u8]>,

    /// Game title.
    game_title: String,

    /// Game code.
    pub(crate) game_code: String,

    /// Maker code.
    #[allow(unused)]
    maker_code: String,
}

// Cartridge header from GBATEK:
//   Address Bytes Expl.
//   000h    4     ROM Entry Point  (32bit ARM branch opcode, eg. "B rom_start")
//   004h    156   Nintendo Logo    (compressed bitmap, required!)
//   0A0h    12    Game Title       (uppercase ascii, max 12 characters)
//   0ACh    4     Game Code        (uppercase ascii, 4 characters)
//   0B0h    2     Maker Code       (uppercase ascii, 2 characters)
//   0B2h    1     Fixed value      (must be 96h, required!)
//   0B3h    1     Main unit code   (00h for current GBA models)
//   0B4h    1     Device type      (usually 00h) (bit7=DACS/debug related)
//   0B5h    7     Reserved Area    (should be zero filled)
//   0BCh    1     Software version (usually 00h)
//   0BDh    1     Complement check (header checksum, required!)
//   0BEh    2     Reserved Area    (should be zero filled)
//   --- Additional Multiboot Header Entries ---
//   0C0h    4     RAM Entry Point  (32bit ARM branch opcode, eg. "B ram_start")
//   0C4h    1     Boot mode        (init as 00h - BIOS overwrites this value!)
//   0C5h    1     Slave ID Number  (init as 00h - BIOS overwrites this value!)
//   0C6h    26    Not used         (seems to be unused)
//   0E0h    4     JOYBUS Entry Pt. (32bit ARM branch opcode, eg. "B joy_start")

impl Rom {
    /// Load a ROM from the bytes of a ROM dump.
    pub fn new(data: &[u8]) -> Rom {
        assert!(data.len() >= 192);

        let game_title = std::str::from_utf8(&data[0xa0..0xac])
            .expect("invalid game title")
            .to_string();
        let game_code = std::str::from_utf8(&data[0xac..0xb0])
            .expect("invalid game title")
            .to_string();
        let maker_code = std::str::from_utf8(&data[0xb0..0xb2])
            .expect("invalid game title")
            .to_string();

        Rom {
            data: data.into(),
            game_title,
            game_code,
            maker_code,
        }
    }

    /// Create an empty ROM (no cartridge).
    pub fn empty() -> Rom {
        Rom {
            data: Box::new([]),
            game_title: "".to_string(),
            game_code: "".to_string(),
            maker_code: "".to_string(),
        }
    }
}

impl std::fmt::Debug for Rom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let backup_type = BackupType::detect(self);
        f.write_fmt(format_args!(
            "Rom(len=0x{:X}, title=\"{}\", code=\"{}\", backup={:?})",
            self.data.len(),
            self.game_title,
            self.game_code,
            backup_type,
        ))
    }
}

impl Default for Rom {
    fn default() -> Self {
        Rom::empty()
    }
}
