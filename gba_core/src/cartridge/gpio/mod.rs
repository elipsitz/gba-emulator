use bit::BitIndex;
use serde::{Deserialize, Serialize};

use crate::cartridge::gpio::rtc::Rtc;

mod rtc;

const REG_DATA: u32 = 0xC4;
const REG_DIRECTION: u32 = 0xC6;
const REG_CONTROL: u32 = 0xC8;

/// State for the GPIO interface in the cartridge (and whatever it's connected to).
#[derive(Serialize, Deserialize)]
pub struct Gpio {
    /// Whether the GPIO registers are readable.
    readable: bool,

    /// The direction for each data bit.
    direction: [GpioDirection; 4],

    /// The device connected to the GPIO.
    /// For now, always RTC.
    /// TODO: see about supporting other devices
    device: Rtc,
}

/// Type of GPIO-connected chip.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GpioType {
    /// Real-time clock (RTC).
    Rtc,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum GpioDirection {
    /// Input to GBA.
    In = 0,
    /// Output from GBA.
    Out = 1,
}

impl Gpio {
    pub fn new(kind: GpioType) -> Gpio {
        // XXX: see if there are other GPIOs to implement.
        assert_eq!(kind, GpioType::Rtc);
        Gpio {
            readable: false,
            direction: [GpioDirection::In; 4],
            device: Rtc::new(),
        }
    }

    /// Read from GPIO.
    ///
    /// Returns None if the GPIO is write only.
    pub fn read(&mut self, addr: u32) -> Option<u16> {
        if !self.readable {
            return None;
        }

        let out = match addr {
            REG_DATA => {
                // XXX: mask it so you only get input pins?
                let data = self.device.pin_read();
                (data as u16) & 0b1111
            }
            REG_DIRECTION => {
                ((self.direction[0] as u16) << 0)
                    | ((self.direction[1] as u16) << 1)
                    | ((self.direction[2] as u16) << 2)
                    | ((self.direction[3] as u16) << 3)
            }
            REG_CONTROL => self.readable as u16,
            _ => 0,
        };
        Some(out)
    }

    /// Write to GPIO.
    pub fn write(&mut self, addr: u32, value: u16) {
        match addr {
            REG_DATA => {
                // XXX: mask it so you only get output pins?
                self.device.pin_write((value & 0b1111) as u8);
            }
            REG_DIRECTION => {
                for i in 0..4 {
                    self.direction[i] = if value.bit(i) {
                        GpioDirection::Out
                    } else {
                        GpioDirection::In
                    }
                }
            }
            REG_CONTROL => {
                self.readable = value.bit(0);
            }
            _ => {}
        }
    }
}
