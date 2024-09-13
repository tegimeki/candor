use crate::Packet;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Main stats for CAN bus/interface
#[derive(Default, Clone)]
pub struct Stats {
    pub baud: u32,
    pub bytes: u32,
    pub packets: u32,
    pub load: u32,
    pub pps: u32,
    messages: VecDeque<Message>,
    ids: HashMap<u32, usize>,
    bytes_accum: u32,
    packet_accum: u32,
}

/// Message stats
#[derive(Default, Clone)]
pub struct Message {
    pub id: u32,
    pub extended: bool,
    pub count: u32,
    pub time: Option<Instant>,
    pub delta: Duration,
    pub missing: Duration,
    pub current: Packet,
    pub previous: Packet,
}

impl Stats {
    pub fn new(baud: u32) -> Self {
        Self {
            baud,
            ..Default::default()
        }
    }

    pub fn messages(&self) -> &VecDeque<Message> {
        &self.messages
    }

    pub fn periodic(&mut self) {
        self.load =
            (self.load + (100 * ((self.bytes_accum * 10) + 5) / self.baud)) / 2;
        self.pps = (self.pps + self.packet_accum) / 2;
        self.bytes_accum = 0;
        self.packet_accum = 0;

        // mark expired data
        for info in self.messages.iter_mut() {
            let now = Instant::now();
            let time = info.time.unwrap_or(now - Duration::from_secs(1));
            let expired = (info.delta * 3).min(Duration::from_secs(2));
            if now - time > expired {
                info.missing = now - time;
                info.delta = Duration::default();
            }
        }
    }

    pub fn packet(&mut self, packet: &Packet) {
        self.packets += 1;
        self.packet_accum += 1;
        let bytes = packet.bytes.len() as u32;
        self.bytes += bytes;
        self.bytes_accum += bytes;

        let index = *self.ids.entry(packet.id).or_insert_with(|| {
            self.messages
                .push_back(Message::new(packet.id, packet.extended));
            self.messages.len() - 1
        });

        let message = self.messages.get_mut(index).expect("index for id");

        message.count += 1;
        message.previous = message.current.clone();
        message.current = packet.clone();

        let time = packet.time.unwrap_or(Instant::now());
        let delta = time - message.time.unwrap_or(time);
        message.delta = delta;
        message.missing = Duration::default();
        message.time = packet.time;
    }
}

impl Message {
    pub fn new(id: u32, extended: bool) -> Self {
        Self {
            id,
            extended,
            ..Default::default()
        }
    }
}
