use crate::{sources::Source, AppEvent, Packet};

use std::{f32, u32, u8};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub struct PeakTraceFile {
    packets: Vec<Packet>,
}

impl PeakTraceFile {
    pub fn new(name: &str, index: usize, sync_time: bool) -> io::Result<Self> {
        let file = File::open(name)?;
        let buf = BufReader::new(file);
        let lines: Vec<String> = buf
            .lines()
            .map(|l| l.expect("Could not parse line"))
            .collect();

        let mut packets: Vec<Packet> = Vec::with_capacity(lines.len());
        let start_time = Instant::now();
        let mut first_time: Option<u64> = None;

        for line in lines.iter() {
            if line.starts_with(";") {
                // TODO: parse the file format version, as
                // each requires different handling; for now
                // we only work with 2.0
                continue;
            }

            let fields: Vec<String> =
                line.split_whitespace().map(|i| i.to_string()).collect();
            if fields.len() < 6 {
                continue;
            }

            // TODO: fix the default non-handling of errors
            let id = u32::from_str_radix(&fields[3], 16).unwrap_or(0);
            let dlc = u32::from_str_radix(&fields[5], 16).unwrap_or(0);
            let time_ms = fields[1].parse::<f32>().unwrap_or(0.0f32);
            let mut time_ns = (time_ms * 1000000.0) as u64;
            let mut bytes: Vec<u8> = Vec::with_capacity(dlc as usize);
            for i in 0..dlc {
                bytes.push(
                    u8::from_str_radix(&fields[6 + i as usize], 16)
                        .unwrap_or(0),
                );
            }

            match first_time {
                None => {
                    first_time = Some(time_ns);
                    if sync_time {
                        time_ns = 0
                    }
                }
                Some(t) => {
                    if sync_time {
                        time_ns -= t
                    }
                }
            }

            let packet = Packet {
                source: index,
                time: Some(start_time + Duration::from_nanos(time_ns)),
                extended: fields[3].len() > 4,
                id,
                bytes,
            };
            packets.push(packet);
        }
        Ok(Self { packets })
    }
}

pub struct PeakTraceSource {
    name: String,
    baud: u32,
}

impl PeakTraceSource {
    pub fn new(
        name: &str,
        index: usize,
        default_baud: u32,
        sync_time: bool,
        tx: mpsc::Sender<AppEvent>,
    ) -> io::Result<Self> {
        let file = PeakTraceFile::new(name, index, sync_time)?;
        thread::spawn(move || {
            let count = file.packets.len();
            let mut index = 0;
            let start_time = Instant::now();
            let mut sleep_time = start_time;
            let mut offset = Duration::default();
            loop {
                let mut packet = file.packets.get(index).unwrap().clone();
                let time = packet.time.unwrap() + offset;
                let delta = time - sleep_time;

                packet.time = Some(Instant::now());

                if tx.send(AppEvent::Packet(packet)).is_err() {
                    println!("Error sending frame event");
                }

                if delta >= Duration::from_millis(0) {
                    thread::sleep(delta);
                    sleep_time = Instant::now();
                }

                index += 1;
                if index >= count {
                    index = 0;
                    offset = Instant::now() - start_time;
                    //                    break; // DEBUG: stop upon wrap
                }
            }
        });
        Ok(Self {
            name: name.to_string(),
            baud: default_baud,
        })
    }
}

impl Source for PeakTraceSource {
    fn name(&self) -> String {
        let path = Path::new(&self.name);
        path.file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap()
            .to_owned()
    }

    fn baud(&self) -> u32 {
        self.baud
    }
}
