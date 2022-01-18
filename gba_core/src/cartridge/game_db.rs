use super::BackupType;

#[derive(Copy, Clone, Debug)]
pub struct DatabaseEntry {
    pub game_code: &'static str,
    pub backup_type: BackupType,
}

macro_rules! entry {
    ($code:literal, $backup_type:ident) => {
        DatabaseEntry {
            game_code: $code,
            backup_type: BackupType::$backup_type,
        }
    };
}

/// The game database.
static DATABASE: &[DatabaseEntry] = &[
    entry!("ALUE", Eeprom512), // Super Monkey Ball Jr. (USA)
];

pub fn lookup(game_code: &str) -> Option<DatabaseEntry> {
    DATABASE.iter().find(|&e| e.game_code == game_code).cloned()
}
