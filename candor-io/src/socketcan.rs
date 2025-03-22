use crate::Source;
use candor::Packet;
use socketcan::{
    BlockingCan, CanInterface, CanSocket, EmbeddedFrame, Frame, Socket,
};
use std::{io, sync::mpsc, thread, time::Instant};

#[derive(Default, Clone)]
pub struct SocketCanSource {
    name: String,
    baud: u32,
}

impl SocketCanSource {
    pub fn new(
        name: &str,
        index: usize,
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
                    source: index,
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
}

impl Source for SocketCanSource {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn baud(&self) -> u32 {
        self.baud
    }
}
