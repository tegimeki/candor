pub mod sources;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_some_tests() {
        todo!("need some tests!")
    }
}
