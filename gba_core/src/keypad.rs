/// Keypad State
#[derive(Copy, Clone, Debug)]
pub struct KeypadState {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub r: bool,
    pub l: bool,
}

impl Default for KeypadState {
    fn default() -> Self {
        KeypadState {
            a: false,
            b: false,
            select: false,
            start: false,
            right: false,
            left: false,
            up: false,
            down: false,
            r: false,
            l: false,
        }
    }
}

impl Into<u16> for KeypadState {
    fn into(self) -> u16 {
        // 0 for pressed, 1 for not pressed.
        ((!self.a as u16) << 0)
            | ((!self.b as u16) << 1)
            | ((!self.select as u16) << 2)
            | ((!self.start as u16) << 3)
            | ((!self.right as u16) << 4)
            | ((!self.left as u16) << 5)
            | ((!self.up as u16) << 6)
            | ((!self.down as u16) << 7)
            | ((!self.r as u16) << 8)
            | ((!self.l as u16) << 9)
    }
}
