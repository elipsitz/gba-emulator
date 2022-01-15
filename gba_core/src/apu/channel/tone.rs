use super::Sequencer;

pub struct ToneChannel {
    pub sequencer: Sequencer,
}

pub enum ToneRegister {
    Sweep,
    Duty,
    Freq,
}

impl ToneChannel {
    pub fn new() -> ToneChannel {
        ToneChannel {
            sequencer: Sequencer::new(),
        }
    }

    pub fn read_register(&mut self, register: ToneRegister) -> u16 {
        0
    }

    pub fn write_register(&mut self, register: ToneRegister, value: u16) {}
}
