use bit::BitIndex;
use chrono::{Datelike, Timelike};

/// Clock Pin
const PIN_SCK: usize = 0;
/// Data Pin
const PIN_SIO: usize = 1;
/// Chip-Select Pin
const PIN_CS: usize = 2;

#[derive(Copy, Clone, Debug)]
enum Register {
    /// Forces time to reset.
    ForceReset = 0,
    /// Control register.
    Control = 4,
    /// DateTime (7 bytes, YMDWHMS)
    DateTime = 2,
    /// Time (3 bytes, HMS)
    Time = 6,
    /// Unused (alarm1), always 0xFF
    Alarm1 = 1,
    /// Unused (alarm2), always 0xFF
    Alarm2 = 5,
    /// Force IRQ
    ForceIrq = 3,
    /// Unused (free), always 0xFF
    Free = 7,
}

#[derive(Copy, Clone, Debug)]
enum State {
    /// Waiting for a command opcode.
    Waiting,
    /// Handling a read register command.
    Read(Register),
    /// Handling a write register command.
    Write(Register),
}

impl Register {
    /// Returns the parameter length in bytes.
    fn param_len(self) -> usize {
        match self {
            Self::ForceReset => 0,
            Self::Control => 1,
            Self::DateTime => 7,
            Self::Time => 3,
            Self::Alarm1 => 1,
            Self::Alarm2 => 1,
            Self::ForceIrq => 0,
            Self::Free => 1,
        }
    }

    fn from_u8(data: u8) -> Self {
        match data {
            0 => Self::ForceReset,
            1 => Self::Alarm1,
            2 => Self::DateTime,
            3 => Self::ForceIrq,
            4 => Self::Control,
            5 => Self::Alarm2,
            6 => Self::Time,
            7 => Self::Free,
            _ => unreachable!(),
        }
    }
}

/// State for the real-time clock.
pub struct Rtc {
    /// Whether chip-select is enabled.
    selected: bool,
    /// Whether the clock pin is high.
    clock: bool,
    /// State of the data pin.
    data: bool,

    /// Current command state.
    state: State,

    /// Serial buffer.
    serial_buffer: [u8; 8],
    /// Number of bits in the serial buffer.
    serial_buffer_len: usize,

    /// Control register: unknown (bit 1) -- "IRQ duty/hold related?"
    control_unknown1: bool,
    /// Control register: per-minute IRQ (bit 3)
    control_irq: bool,
    /// Control register: unknown (bit 5)
    control_unknown2: bool,
    /// Control register: 24-hour mode (bit 6). True for 24H, false for 12H.
    control_24h: bool,
}

impl Rtc {
    pub fn new() -> Rtc {
        Rtc {
            selected: false,
            clock: false,
            data: false,

            state: State::Waiting,

            serial_buffer: [0; 8],
            serial_buffer_len: 0,

            control_unknown1: false,
            control_irq: false,
            control_unknown2: false,
            control_24h: false,
        }
    }

    /// Called when GPIO pins are set.
    pub fn pin_write(&mut self, pins: u8) {
        let pin_clock = pins.bit(PIN_SCK);
        let pin_data = pins.bit(PIN_SIO);
        let pin_chip_select = pins.bit(PIN_CS);

        // Handle chip-select.
        if !self.selected {
            if pin_chip_select {
                self.selected = true;
                // println!("rtc: chip selected!");
            }
            return;
        }
        if !pin_chip_select {
            self.selected = false;
            self.reset_serial();
            // println!("rtc: chip unselected.");
            return;
        }

        // Only do something on rising clock edge.
        let rising_clock = pin_clock && !self.clock;
        self.clock = pin_clock;
        if !rising_clock {
            return;
        }

        // Handle the clock.
        // println!("Clock: state={:?}", self.state);
        match self.state {
            State::Waiting => {
                self.data = pin_data;
                if self.serial_read(1) {
                    // We got the command code.
                    let byte = self.serial_buffer[0];
                    self.reset_serial();

                    // Bit swap if necessary. We expect the bottom bits to be 0110.
                    let byte = if byte & 0x0F == 0b0110 {
                        byte
                    } else {
                        byte.reverse_bits()
                    };
                    let register = Register::from_u8(byte.bit_range(4..7));

                    match byte.bit(7) {
                        true => {
                            // Reading a register.
                            self.register_read(register);
                            if register.param_len() > 0 {
                                self.serial_buffer_len = register.param_len() * 8;
                                self.state = State::Read(register);
                            } else {
                                self.state = State::Waiting;
                            }
                        }
                        false => {
                            // Writing a register.
                            if register.param_len() > 0 {
                                self.state = State::Write(register);
                            } else {
                                self.register_write(register);
                                self.state = State::Waiting;
                            }
                        }
                    }
                }
            }
            State::Read(register) => {
                // Output bits, LSB first.
                let bit = (register.param_len() * 8) - self.serial_buffer_len;
                self.data = self.serial_buffer[bit / 8].bit(bit % 8);

                self.serial_buffer_len -= 1;
                if self.serial_buffer_len == 0 {
                    self.state = State::Waiting;
                }
            }
            State::Write(register) => {
                // Wait until we have enough bits...
                self.data = pin_data;
                if self.serial_read(register.param_len()) {
                    self.register_write(register);
                    self.reset_serial();
                    self.state = State::Waiting;
                }
            }
        }
    }

    /// Read a register, filling up the serial buffer.
    fn register_read(&mut self, register: Register) {
        // println!("rtc: read from {:?}", register);
        match register {
            Register::Control => {
                let mut data = 0u8;
                data.set_bit(1, self.control_unknown1);
                data.set_bit(3, self.control_irq);
                data.set_bit(5, self.control_unknown2);
                data.set_bit(6, self.control_24h);
                self.serial_buffer[0] = data;
            }
            Register::DateTime => {
                let datetime = DateTime::now();
                self.serial_buffer[0] = datetime.year();
                self.serial_buffer[1] = datetime.month();
                self.serial_buffer[2] = datetime.day();
                self.serial_buffer[3] = datetime.day_of_week();
                self.serial_buffer[4] = datetime.hour(self.control_24h);
                self.serial_buffer[5] = datetime.minute();
                self.serial_buffer[6] = datetime.second();
            }
            Register::Time => {
                let datetime = DateTime::now();
                self.serial_buffer[0] = datetime.hour(self.control_24h);
                self.serial_buffer[1] = datetime.minute();
                self.serial_buffer[2] = datetime.second();
            }
            _ => {}
        }
    }

    /// Write to a register, using the serial buffer.
    fn register_write(&mut self, register: Register) {
        // println!("rtc: write to {:?}, {:?}", register, self.serial_buffer);
        match register {
            Register::Control => {
                let data = self.serial_buffer[0];
                self.control_unknown1 = data.bit(1);
                self.control_irq = data.bit(3);
                self.control_unknown2 = data.bit(5);
                self.control_24h = data.bit(6);
            }
            Register::ForceReset => {
                // Reset the date and time to 2000-01-01 00:00:00?
                println!("RTC: unimplemented force reset");
                self.control_unknown1 = false;
                self.control_irq = false;
                self.control_unknown2 = false;
                self.control_24h = false;
            }
            Register::ForceIrq => {
                // TODO: support cartridge IRQ
                println!("RTC: unimplemented force IRQ");
            }
            // XXX: support changing the time?
            _ => {}
        }
    }

    /// Called when GPIO pins are read.
    pub fn pin_read(&mut self) -> u8 {
        (self.data as u8) << PIN_SIO
    }

    /// Adds the current data bit to the serial buffer.
    /// Then, returns whether the number of buffered bytes is the requested number.
    fn serial_read(&mut self, requested: usize) -> bool {
        let byte_index = self.serial_buffer_len / 8;
        let bit_index = self.serial_buffer_len % 8;
        self.serial_buffer[byte_index].set_bit(bit_index, self.data);
        self.serial_buffer_len += 1;

        self.serial_buffer_len == (requested * 8)
    }

    /// Resets the serial buffer.
    fn reset_serial(&mut self) {
        self.serial_buffer_len = 0;
    }
}

struct DateTime(chrono::DateTime<chrono::Local>);

impl DateTime {
    /// Get the current DateTime.
    fn now() -> DateTime {
        // XXX: consider allowing configuring fixed or offset time.
        DateTime(chrono::Local::now())
    }

    fn year(&self) -> u8 {
        // (BCD 00h..99h = 2000..2099)
        encode_bcd((self.0.year() % 100) as u8)
    }

    fn month(&self) -> u8 {
        // (BCD 01h..12h = January..December)
        encode_bcd(self.0.month() as u8)
    }

    fn day(&self) -> u8 {
        // (BCD 01h..28h,29h,30h,31h, range depending on month/year)
        encode_bcd(self.0.day() as u8)
    }

    fn day_of_week(&self) -> u8 {
        // 1 is Monday (in Emerald).
        encode_bcd(self.0.weekday().number_from_monday() as u8)
    }

    fn hour(&self, is_24h: bool) -> u8 {
        // (BCD 00h..23h in 24h mode, or 00h..11h in 12h mode)
        // bit 7: 0 if hour is <= 11, 1 otherwise.
        let hour = if is_24h {
            self.0.hour() as u8
        } else {
            (self.0.hour() % 12) as u8
        };
        let pm = self.0.hour() >= 12;
        encode_bcd(hour) | ((pm as u8) << 7)
    }

    fn minute(&self) -> u8 {
        // (BCD 00h..59h)
        encode_bcd(self.0.minute() as u8)
    }

    fn second(&self) -> u8 {
        // (BCD 00h..59h)
        encode_bcd(self.0.second() as u8)
    }
}

/// Converts a regular number (range 0..=99) to binary coded decimal.
fn encode_bcd(input: u8) -> u8 {
    assert!(input < 100);
    let ones = input % 10;
    let tens = input / 10;
    ones | (tens * 16)
}
