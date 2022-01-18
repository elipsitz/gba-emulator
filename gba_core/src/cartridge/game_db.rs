use super::{BackupType, GpioType};

#[derive(Copy, Clone, Debug)]
pub struct DatabaseEntry {
    pub game_code: &'static str,
    pub backup_type: BackupType,
    pub gpio_type: Option<GpioType>,
}

macro_rules! entry {
    ($code:literal, $backup_type:ident, None) => {
        DatabaseEntry {
            game_code: $code,
            backup_type: BackupType::$backup_type,
            gpio_type: None,
        }
    };
    ($code:literal, $backup_type:ident, $gpio_type:ident) => {
        DatabaseEntry {
            game_code: $code,
            backup_type: BackupType::$backup_type,
            gpio_type: Some(GpioType::$gpio_type),
        }
    };
}

/// The game database.
static DATABASE: &[DatabaseEntry] = &[
    entry!("ALUE", Eeprom512, None), // Super Monkey Ball Jr. (USA)
    entry!("AXVE", Flash128K, Rtc),  // Pokemon - Ruby Version (USA, Europe)
    entry!("AXPE", Flash128K, Rtc),  // Pokemon - Sapphire Version (USA, Europe)
    entry!("BPEE", Flash128K, Rtc),  // Pokemon - Emerald Version (USA, Europe)
    entry!("BPRE", Flash128K, None), // Pokemon - Fire Red Version (USA, Europe)
    entry!("BPGE", Flash128K, None), // Pokemon - Leaf Green Version (USA, Europe)
];

pub fn lookup(game_code: &str) -> Option<DatabaseEntry> {
    DATABASE.iter().find(|&e| e.game_code == game_code).cloned()
}
