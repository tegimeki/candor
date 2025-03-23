//! CANdor library for CAN bus decoding/observation/reverse-engineering

pub mod stats;

use std::time::Instant;

#[derive(Default, Clone)]
pub struct Packet {
    pub source: usize,
    pub time: Option<Instant>,
    pub extended: bool,
    pub id: u32,
    pub bytes: Vec<u8>,
}

impl Packet {
    pub fn id_string(&self) -> String {
        if self.extended {
            format!("{:08X} ", self.id)
        } else {
            format!("     {:03X} ", self.id)
        }
    }
}
