use crate::Packet;
use bitvec::prelude::*;
use can_dbc::{ByteOrder, MultiplexIndicator, ValueType, DBC};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fs::File;
use std::io;
use std::io::prelude::*;
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
    dbcs: Vec<DBC>,
}

/// Message stats
#[derive(Default, Clone)]
pub struct Message {
    pub source: usize,
    pub dbc: Option<usize>,
    pub id: u32,
    pub expanded: bool,
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

    pub fn add_dbc(&mut self, filename: String) -> io::Result<()> {
        let mut f = File::open(filename)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        let dbc = DBC::from_slice(&buffer).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{:?}", e),
            )
        })?;
        self.dbcs.push(dbc);
        Ok(())
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

        // register messages as they are seen
        let index = *self.ids.entry(packet.id).or_insert_with(|| {
            // figure out if message is in one of the DBCs
            let mut found: Option<usize> = None;
            for (index, dbc) in self.dbcs.iter().enumerate() {
                let msg = dbc
                    .messages()
                    .iter()
                    .find(|&m| m.message_id().raw() == packet.id);
                if msg.is_some() {
                    found = Some(index);
                    break;
                }
            }
            self.messages.push_back(Message::new(packet, found));
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

    pub fn dbc_message(&self, message: &Message) -> Option<&can_dbc::Message> {
        match message.dbc {
            Some(dbc) => self
                .dbcs
                .get(dbc)
                .unwrap()
                .messages()
                .iter()
                .find(|&m| m.message_id().raw() == message.id),
            None => None,
        }
    }

    // TODO: move into a decode module (and handle value tables, etc.)
    pub fn signal_text(
        &self,
        msg: &can_dbc::Message,
        sig: &can_dbc::Signal,
        packet: &Packet,
    ) -> String {
        let start = *sig.start_bit() as usize;
        let size = *sig.signal_size() as usize;

        if *sig.multiplexer_indicator() != MultiplexIndicator::Plain
            && *sig.multiplexer_indicator() != MultiplexIndicator::Multiplexor
        {
            return "".to_string();
        }

        let bytes = packet.bytes.as_slice();
        let raw = match sig.byte_order() {
            ByteOrder::LittleEndian => {
                bytes.view_bits::<Lsb0>()[start..start + size].load_le::<u64>()
            }
            ByteOrder::BigEndian => bytes.view_bits::<Msb0>()
                [(start - (size - 1))..start + 1]
                .load_be::<u64>(),
        };

        let value = match *sig.value_type() {
            ValueType::Unsigned => raw as f32,
            ValueType::Signed => i64::from_ne_bytes(raw.to_ne_bytes()) as f32,
        };

        let factor = *sig.factor() as f32;
        let offset = *sig.offset() as f32;
        if factor != 1.0 || offset < 0.0 {
            format!("{:.3}{}", value * factor + offset, sig.unit())
        } else {
            format!("{}{}", (value + offset) as u64, sig.unit())
        }
    }
}

impl Message {
    pub fn new(packet: &Packet, dbc: Option<usize>) -> Self {
        Self {
            source: packet.source,
            dbc,
            id: packet.id,
            expanded: true,
            extended: packet.extended,
            ..Default::default()
        }
    }
}
