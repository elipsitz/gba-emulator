use bit::BitIndex;
use serde::{Deserialize, Serialize};

use crate::{Gba, InterruptKind};

/// Keypad State
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
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

impl Gba {
    /// Set the current keypad state.
    pub fn set_keypad_state(&mut self, state: KeypadState) {
        self.keypad_state = state;

        // Check for interrupt.
        let keycnt = self.io.keycnt;
        let irq_enabled = self.io.keycnt.bit(14);
        if irq_enabled {
            // False: logical OR. True: logical AND.
            let irq_condition = self.io.keycnt.bit(15);
            let pressed = Into::<u16>::into(state) & 0x3FF;
            let mask = keycnt & 0x3FF;
            if mask != 0 {
                let fire = if irq_condition {
                    // Logical AND: all masked buttons must be pressed.
                    (mask & pressed) == mask
                } else {
                    // Logical OR: any of the masked buttons must be pressed.
                    (mask & pressed) != 0
                };
                if fire {
                    self.interrupt_raise(InterruptKind::Keypad);
                }
            }
        }
    }
}
