use crate::Packet;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Default, Clone)]
pub struct Stats {
    pub data: u32,
    pub accum: u32,
    pub bytes: u32,
    pub packets: u32,
    pub load: u32,
    pub pps: u32,
}

#[derive(Default, Clone)]
pub struct Bucket {
    pub id: u32,
    pub extended: bool,
    pub count: u32,
    pub time: Option<Instant>,
    pub delta: Duration,
    pub missing: Duration,
    pub current: Packet,
    pub previous: Packet,
}

impl Bucket {
    pub fn new(id: u32, extended: bool) -> Self {
        Self {
            id,
            extended,
            ..Default::default()
        }
    }
}

impl Stats {
    pub fn periodic(&mut self, buckets: &mut VecDeque<Bucket>, baud: u32) {
        self.load = (self.load + (100 * ((self.bytes * 10) + 5) / baud)) / 2;
        self.pps = (self.pps + self.packets) / 2;
        self.bytes = 0;
        self.packets = 0;

        // mark expired data
        for info in buckets.iter_mut() {
            let now = Instant::now();
            let time = info.time.unwrap_or(now - Duration::from_secs(1));
            let expired = (info.delta * 3).min(Duration::from_secs(2));
            if now - time > expired {
                info.missing = now - time;
                info.delta = Duration::default();
            }
        }
    }

    pub fn packet(&mut self, bucket: &mut Bucket, packet: &Packet) {
        bucket.count += 1;
        bucket.previous = bucket.current.clone();
        bucket.current = packet.clone();

        let time = packet.time.unwrap_or(Instant::now());
        let delta = time - bucket.time.unwrap_or(time);
        bucket.delta = delta;
        bucket.missing = Duration::default();
        bucket.time = packet.time;

        self.packets += 1;
        self.data += 1;
        self.bytes += packet.bytes.len() as u32;
    }
}
