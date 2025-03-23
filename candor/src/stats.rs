use crate::Packet;
use bitvec::prelude::*;
use can_dbc::{ByteOrder, DBC, MessageId, MultiplexIndicator, ValueType};
use std::collections::{BTreeMap, BinaryHeap, HashMap, VecDeque};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::time::{Duration, Instant};

/// Main stats for CAN bus/interface
#[derive(Default, Clone)]
pub struct Stats {
    /// Baud rate used to compute bus load
    pub baud: u32,
    /// Total number of bytes received
    pub bytes: u32,
    /// Total number of packets received
    pub packets: u32,
    /// Computed bus load
    pub load: u32,
    /// Packets per second
    pub pps: u32,

    messages: VecDeque<Message>,
    ids: HashMap<u32, usize>,
    bytes_accum: u32,
    packet_accum: u32,
    dbcs: Vec<DbcLookup>,
    sorted: bool,
    ordering: Vec<usize>,
    time: Option<Instant>,
}

/// Message stats
#[derive(Default, Clone)]
pub struct Message {
    pub source: usize,
    pub dbc: Option<usize>,
    pub count: usize,
    pub delta: Duration,
    pub missing: Duration,
    pub current: Packet,
    pub previous: Packet,
    count_accum: usize,
}

/// Helper for looking up DBC messages by ID
#[derive(Clone)]
struct DbcLookup {
    dbc: DBC,
    ids: BTreeMap<u32, usize>,
}

impl DbcLookup {
    fn new(dbc: DBC) -> Self {
        // get a map of message IDs to their corresponding index
        let mut ids: BTreeMap<u32, usize> = Default::default();
        for (index, message) in dbc.messages().iter().enumerate() {
            let id = match *message.message_id() {
                MessageId::Standard(id) => id as u32,
                MessageId::Extended(id) => id,
            };
            ids.insert(id, index);
        }
        Self { dbc, ids }
    }
}

impl Stats {
    pub fn new(baud: u32) -> Self {
        Self {
            baud,
            time: Some(Instant::now()),
            ..Default::default()
        }
    }

    pub fn add_dbc(&mut self, filename: String) -> io::Result<()> {
        let mut f = File::open(filename)?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        let dbc = DBC::from_slice(&buffer).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("{:?}", e))
        })?;
        self.dbcs.push(DbcLookup::new(dbc));
        Ok(())
    }

    pub fn messages(&self) -> &VecDeque<Message> {
        &self.messages
    }

    pub fn ordering(&self) -> &Vec<usize> {
        &self.ordering
    }

    pub fn periodic(&mut self) {
        let now = Instant::now();
        let last = self.time.unwrap_or(now);

        if (now - last).as_secs() < 1 {
            return;
        }
        self.time = Some(now);

        // TOD: improve this very loose estimate
        self.load =
            (self.load + (100 * ((self.bytes_accum * 10) + 5) / self.baud)) / 2;
        self.pps = (self.pps + self.packet_accum) / 2;
        self.bytes_accum = 0;
        self.packet_accum = 0;

        // mark expired data
        for message in self.messages.iter_mut() {
            if message.count_accum > 0 {
                let period = 1000 / message.count_accum;
                message.delta = Duration::from_millis(period as u64);
                message.count_accum = 0;
            } else {
                let time = message
                    .current
                    .time
                    .unwrap_or(now - Duration::from_secs(1));
                let expired = (message.delta * 3).min(Duration::from_secs(2));
                if now - time > expired {
                    message.missing = now - time;
                    message.delta = Duration::default();
                }
            }
        }
    }

    pub fn process_packet(&mut self, packet: &Packet) {
        self.packets += 1;
        self.packet_accum += 1;

        let bytes = packet.bytes.len() as u32;
        self.bytes += bytes;
        self.bytes_accum += bytes;

        // register messages as they are seen
        let index = *self.ids.entry(packet.id).or_insert_with(|| {
            let dbc = self
                .dbcs
                .iter()
                .enumerate()
                .find(|(_, m)| m.ids.contains_key(&packet.id))
                .map(|(i, _)| i);

            self.messages.push_back(Message::new(packet, dbc));
            self.sorted = false;

            self.messages.len() - 1
        });

        let message = self.messages.get_mut(index).expect("index for id");

        message.count += 1;
        message.count_accum += 1;
        message.previous = message.current.clone();
        message.current = packet.clone();

        message.missing = Duration::default();

        if !self.sorted {
            let mut heap: BinaryHeap<u32> = BinaryHeap::new();
            self.ordering.resize(self.messages.len(), 0);
            for message in self.messages.iter() {
                heap.push(message.current.id);
            }
            for index in (0..self.ordering.len()).rev() {
                let e = *self.ids.entry(heap.pop().unwrap()).or_default();
                self.ordering[index] = e;
            }
            self.sorted = true;
        }
    }

    pub fn dbc_message(&self, message: &Message) -> Option<&can_dbc::Message> {
        if let Some(dbc) = message.dbc {
            if let Some(lookup) = self.dbcs.get(dbc) {
                if let Some(index) = lookup.ids.get(&message.current.id) {
                    return lookup.dbc.messages().get(*index);
                }
            }
        }

        None
    }

    pub fn signal_text(
        &self,
        _msg: &can_dbc::Message,
        sig: &can_dbc::Signal,
        packet: &Packet,
    ) -> String {
        let start = *sig.start_bit() as usize;
        let size = *sig.signal_size() as usize;

        if *sig.multiplexer_indicator() != MultiplexIndicator::Plain
            && *sig.multiplexer_indicator() != MultiplexIndicator::Multiplexor
        {
            // TODO: support multiplexed messages
            return "<multiplexed>".to_string();
        }

        let bytes = packet.bytes.as_slice();
        let value = match *sig.value_type() {
            ValueType::Unsigned => {
                let raw = match sig.byte_order() {
                    ByteOrder::LittleEndian => bytes.view_bits::<Lsb0>()
                        [start..start + size]
                        .load_le::<u64>(),
                    ByteOrder::BigEndian => bytes.view_bits::<Msb0>()
                        [(start - (size - 1))..start + 1]
                        .load_be::<u64>(),
                };
                raw as f32
            }
            ValueType::Signed => {
                let raw = match sig.byte_order() {
                    ByteOrder::LittleEndian => bytes.view_bits::<Lsb0>()
                        [start..start + size]
                        .load_le::<i64>(),
                    ByteOrder::BigEndian => bytes.view_bits::<Msb0>()
                        [(start - (size - 1))..start + 1]
                        .load_be::<i64>(),
                };
                i64::from_ne_bytes(raw.to_ne_bytes()) as f32
            }
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
            ..Default::default()
        }
    }
}
