//! CANdor library for CAN bus decoding/observation/reverse-engineering

pub mod popup;
pub mod sources;
pub mod stats;

use ratatui::crossterm::event::KeyEvent;
use std::time::Instant;

#[derive(Default, Clone)]
pub struct Packet {
    pub source: usize,
    pub time: Option<Instant>,
    pub extended: bool,
    pub id: u32,
    pub bytes: Vec<u8>,
}

pub enum AppEvent {
    Packet(Packet),
    Key(KeyEvent),
}
