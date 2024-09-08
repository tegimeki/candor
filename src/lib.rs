pub mod stats;

use socketcan::{
    BlockingCan, CanFrame, CanInterface, CanSocket, EmbeddedFrame, Frame,
    Socket,
};
use std::{io, sync::mpsc, thread, time::Instant};

#[derive(Default, Clone)]
pub struct Packet {
    pub source: usize,
    pub time: Option<Instant>,
    pub extended: bool,
    pub id: u32,
    pub bytes: Vec<u8>,
}

#[derive(Default, Clone)]
pub struct Source {
    name: String,
    baud: u32,
}

impl Source {
    pub fn new(
        name: &str,
        source: usize,
        default_baud: u32,
        tx: mpsc::Sender<Packet>,
    ) -> io::Result<Self> {
        let iface = CanInterface::open(name)?;
        let bit_rate = iface.bit_rate();
        let baud = if bit_rate.is_ok() {
            bit_rate.unwrap().unwrap_or(default_baud)
        } else {
            default_baud
        };

        let mut rx = CanSocket::open(name)?;

        thread::spawn(move || {
            while let Ok(res) = rx.receive() {
                let packet = Packet {
                    source,
                    time: Some(Instant::now()),
                    extended: res.is_extended(),
                    id: res.raw_id(),
                    bytes: res.data().to_vec(),
                };
                if tx.send(packet).is_err() {
                    println!("Error sending frame event");
                }
            }
        });

        Ok(Self {
            name: name.to_string(),
            baud,
        })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn baud(&self) -> u32 {
        self.baud
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_some_tests() {
        todo!("need some tests!")
    }
}
