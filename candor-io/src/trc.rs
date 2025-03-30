use crate::Source;
use candor::Packet;

use std::{f32, u32, u8};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use std::error::Error;

pub struct TrcSource {
    name: String,
    baud: u32,
}

impl TrcSource {
    pub fn new(
        name: &str,
        index: usize,
        default_baud: u32,
        sync_time: bool,
        tx: mpsc::Sender<Packet>,
    ) -> Result<Self, Box<dyn Error>> {
        let file = TrcParser::new_from_file(name, index, sync_time)?;
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

                if tx.send(packet).is_err() {
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

impl Source for TrcSource {
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

#[derive(Eq, PartialEq, Debug)]
pub enum TrcVersion {
    V1_0,
    V1_1,
    V1_3,
    V2_0,
    V2_1,
}

#[allow(dead_code)]
pub struct TrcParser {
    packets: Vec<Packet>,
    version: TrcVersion,
}

impl TrcParser {
    pub fn new_from_file(
        filename: &str,
        index: usize,
        sync_time: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let file = File::open(filename)?;
        let buf = BufReader::new(file);
        let lines: Vec<String> = buf
            .lines()
            .map(|l| l.expect("Could not parse line"))
            .collect();
        Self::new_from_lines(lines, index, sync_time)
    }

    pub fn new_from_text(
        text: &str,
        index: usize,
        sync_time: bool,
    ) -> Result<Self, Box<dyn Error>> {
        Self::new_from_lines(
            text.split("\n").map(|s| s.to_string()).collect::<Vec<_>>(),
            index,
            sync_time,
        )
    }

    pub fn new_from_lines(
        lines: Vec<String>,
        index: usize,
        sync_time: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let mut packets: Vec<Packet> = Vec::with_capacity(lines.len());
        let start_time = Instant::now();
        let mut first_time: Option<u64> = None;

        let mut version = TrcVersion::V1_0;
        let mut columns = String::new();
        for line in lines.into_iter() {
            // process directives
            if line.starts_with(";$") {
                let s: Vec<_> = line.split("=").collect();
                if s.len() < 2 {
                    return Err(format!("Invalid directive {line}").into());
                }
                let value = s[1];
                match s[0] {
                    ";$FILEVERSION" => {
                        version = match value {
                            "1.1" => TrcVersion::V1_1,
                            "1.3" => TrcVersion::V1_3,
                            "2.0" => TrcVersion::V2_0,
                            "2.1" => TrcVersion::V2_1,
                            _ => return Err("Unknown version".into()),
                        };
                    }
                    ";$STARTTIME" => {
                        // TODO: parse start time
                    }
                    ";$COLUMNS" => {
                        columns = s[1].to_string();
                    }
                    _ => {} // TODO: error on unrecognized directive?
                }
            } else if !line.starts_with(";") {
                // process trace data packets
                let cols: Vec<String> =
                    line.split_whitespace().map(|i| i.to_string()).collect();

                if cols.is_empty() {
                    continue;
                }

                fn float_ns(s: &str) -> Result<u64, Box<dyn Error>> {
                    let ms = s.parse::<f32>()? as u64;
                    Ok(ms * 1000000)
                }

                fn int_ns(s: &str) -> Result<u64, Box<dyn Error>> {
                    Ok(u64::from_str_radix(s, 16)? * 1000000)
                }

                let (id_col, dlc_col, time_ns) = match version {
                    // 1.x
                    TrcVersion::V1_0 => (2, 3, int_ns(&cols[1])?),
                    TrcVersion::V1_1 => (3, 4, float_ns(&cols[1])?),
                    TrcVersion::V1_3 => (4, 6, float_ns(&cols[1])?),
                    // 2.x
                    TrcVersion::V2_0 | TrcVersion::V2_1 => {
                        let has_bus = columns.contains("B");
                        let has_rsvd = columns.contains("R");
                        let id = if has_bus { 4 } else { 3 };
                        let dlc = match (has_bus, has_rsvd) {
                            (false, false) => 5,
                            (false, true) => 6,
                            (true, false) => 6,
                            (true, true) => 7,
                        };
                        if cols.len() < 6
                            || cols[dlc + 1] == "RTR"
                            || (cols[2] != "DT" && cols[2] != "FD")
                        {
                            continue;
                        }
                        (id, dlc, float_ns(&cols[1])?)
                    }
                };

                let time_ns = match first_time {
                    None => {
                        first_time = Some(time_ns);
                        if sync_time {
                            0
                        } else {
                            time_ns
                        }
                    }
                    Some(t) => {
                        if sync_time {
                            time_ns - t
                        } else {
                            time_ns
                        }
                    }
                };

                let id = u32::from_str_radix(&cols[id_col], 16)?;
                if id == 0xffffffff {
                    continue;
                }

                let dlc = usize::from_str_radix(&cols[dlc_col], 16)?;
                let data_col = dlc_col + 1;
                if cols.len() < data_col + dlc
                    || (dlc > 0 && cols[data_col] == "RTR")
                {
                    continue;
                }

                let mut bytes: Vec<u8> = Vec::with_capacity(dlc);
                for i in 0..dlc {
                    bytes.push(
                        u8::from_str_radix(&cols[data_col + i], 16)
                            .unwrap_or(0),
                    );
                }

                packets.push(Packet {
                    source: index,
                    time: Some(start_time + Duration::from_nanos(time_ns)),
                    extended: cols[id_col].len() > 4,
                    id,
                    bytes,
                });
            }
        }

        Ok(Self { packets, version })
    }
}

// Test data from https://github.com/hardbyte/python-can/tree/main/test/data
// (with some changes to exercise edge cases)
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn version_1_0() {
        let trc = r#"
;##########################################################################
;   C:\some_file.trc
;
;    CAN activities imported from C:\some_file.trc
;    Start time: 13.11.2020 09:31:11.191
;    PCAN-Net: N/A
;    Generated by PEAK-Converter Version 2.2.4.136
;
;    Columns description:
;    ~~~~~~~~~~~~~~~~~~~~~
;    +-current number in actual sample
;    |       +time offset of message (ms)
;    |       |         +ID of message (hex)
;    |       |         |     +data length code
;    |       |         |     |   +data bytes (hex) ...
;    |       |         |     |   |
;----+-   ---+---  ----+---  +  -+ -- -- ...
     1)     17535  00000101  8  00 00 00 00 00 00 00 00
     2)     17540  FFFFFFFF  4  00 00 00 08 -- -- -- -- BUSHEAVY
     3)     17700  00000100  8  00 00 00 00 00 00 00 00
     4)     17873  00000100  8  00 00 00 00 00 00 00 00
     5)     19295      0000  8  00 00 00 00 00 00 00 00
     6)     19500      0000  8  00 00 00 00 00 00 00 00
     7)     19705      0000  8  00 00 00 00 00 00 00 00
     8)     20592  00000100  8  00 00 00 00 00 00 00 00
     9)     20798  00000100  8  00 00 00 00 00 00 00 00
    10)     20956  00000100  8  00 00 00 00 00 00 00 00
    11)     21097  00000100  8  00 00 00 00 00 00 00 00
"#;
        let data = TrcParser::new_from_text(trc, 0, false);
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.version, TrcVersion::V1_0);
        assert_eq!(data.packets.len(), 10);
        assert_eq!(data.packets[0].id, 0x101);
        assert!(data.packets[0].extended);
        assert_eq!(data.packets[4].id, 0x0);
        assert!(!data.packets[4].extended);
    }

    #[test]
    fn version_1_1() {
        let trc = r#"
;$FILEVERSION=1.1
;$STARTTIME=44548.6028595139
;
;   Start time: 18.12.2021 14:28:07.062.0
;   Generated by PCAN-View v5.0.0.814
;
;   Message Number
;   |         Time Offset (ms)
;   |         |        Type
;   |         |        |        ID (hex)
;   |         |        |        |     Data Length
;   |         |        |        |     |   Data Bytes (hex) ...
;   |         |        |        |     |   |
;---+--   ----+----  --+--  ----+---  +  -+ -- -- -- -- -- -- --
     1)     17535.4  Tx     00000100  8  00 00 00 00 00 00 00 00 
     2)     17540.3  Warng  FFFFFFFF  4  00 00 00 08  BUSHEAVY 
     3)     17700.3  Tx     00000100  8  00 00 00 00 00 00 00 00 
     4)     17873.8  Tx     00000100  8  00 00 00 00 00 00 00 00 
     5)     19295.4  Tx         0000  8  00 00 00 00 00 00 00 00 
     6)     19500.6  Tx         0000  8  00 00 00 00 00 00 00 00 
     7)     19705.2  Tx         0000  8  00 00 00 00 00 00 00 00 
     8)     20592.7  Tx     00000100  8  00 00 00 00 00 00 00 00 
     9)     20798.6  Tx     00000100  8  00 00 00 00 00 00 00 00 
    10)     20956.0  Tx     00000100  8  00 00 00 00 00 00 00 00 
    11)     21097.1  Tx     00000100  8  00 00 00 00 00 00 00 00 
"#;
        let data = TrcParser::new_from_text(trc, 0, false);
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.version, TrcVersion::V1_1);
        assert_eq!(data.packets.len(), 10);
        assert_eq!(data.packets[0].id, 0x100);
        assert!(data.packets[0].extended);
        assert_eq!(data.packets[4].id, 0x0);
        assert!(!data.packets[4].extended);
    }

    #[test]
    fn version_1_3() {
        let trc = r#"
;$FILEVERSION=1.3
;$STARTTIME=44548.6028595139
;
;   C:\test.trc
;   Start time: 18.12.2021 14:28:07.062.0
;   Generated by PCAN-Explorer v5.4.0
;-------------------------------------------------------------------------------
;   Bus  Name   Connection                 Protocol  Bit rate
;   1    PCAN   Untitled@pcan_usb          CAN       500 kbit/s
;   2    PTCAN  PCANLight_USB_16@pcan_usb  CAN
;-------------------------------------------------------------------------------
;   Message Number
;   |         Time Offset (ms)
;   |         |       Bus
;   |         |       |    Type
;   |         |       |    |       ID (hex)
;   |         |       |    |       |    Reserved
;   |         |       |    |       |    |   Data Length Code
;   |         |       |    |       |    |   |    Data Bytes (hex) ...
;   |         |       |    |       |    |   |    |
;   |         |       |    |       |    |   |    |
;---+-- ------+------ +- --+-- ----+--- +- -+-- -+ -- -- -- -- -- -- --
     1)        17535.4 1  Tx    00000103 -  8    00 00 00 00 00 00 00 00
     2)        17700.3 1  Tx    00000100 -  8    00 00 00 00 00 00 00 00
     3)        17873.8 1  Tx    00000101 -  0
     4)        19295.4 1  Tx        0000 -  8    00 00 00 00 00 00 00 00
     5)        19500.6 1  Tx        0000 -  8    00 00 00 00 00 00 00 00
     6)        19705.2 1  Tx        0000 -  8    00 00 00 00 00 00 00 00
     7)        20592.7 1  Tx    00000100 -  8    00 00 00 00 00 00 00 00
     8)        20798.6 1  Tx    00000100 -  8    00 00 00 00 00 00 00 00
     9)        20956.0 1  Tx    00000100 -  8    55 00 00 00 00 00 00 00
    10)        21097.1 1  Tx    00000100 -  8    00 00 00 00 00 00 00 00
"#;
        let data = TrcParser::new_from_text(trc, 0, false);
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.version, TrcVersion::V1_3);
        assert_eq!(data.packets.len(), 10);
        assert_eq!(data.packets[0].id, 0x103);
        assert!(data.packets[0].extended);
        assert_eq!(data.packets[2].bytes.len(), 0);
        assert_eq!(data.packets[4].id, 0x0);
        assert!(!data.packets[4].extended);
        assert_eq!(data.packets[8].bytes[0], 0x55);
    }

    #[test]
    fn version_2_0() {
        let trc = r#"
;$FILEVERSION=2.0
;$STARTTIME=44548.6028595139
;$COLUMNS=N,O,T,I,d,l,D
;
;   C:\Users\User\Desktop\python-can\test\data\test_CanMessage_V2_0_BUS1.trc
;   Start time: 18.12.2021 14:28:07.062.001
;   Generated by PEAK-Converter Version 2.2.4.136
;   Data imported from C:\Users\User\Desktop\python-can\test\data\test_CanMessage_V1_1.trc
;-------------------------------------------------------------------------------
;   Connection                 Bit rate
;   N/A                        N/A      
;-------------------------------------------------------------------------------
;   Message   Time    Type ID     Rx/Tx
;   Number    Offset  |    [hex]  |  Data Length
;   |         [ms]    |    |      |  |  Data [hex] ...
;   |         |       |    |      |  |  | 
;---+-- ------+------ +- --+----- +- +- +- -- -- -- -- -- -- --
      1     17535.400 DT 00000100 Tx 8  00 00 00 00 00 00 00 00
      2     17540.300 ST          Rx    00 00 00 08
      3     17700.300 DT 00000100 Tx 8  00 00 00 00 00 00 00 00
      4     17873.800 DT 00000100 Tx 8  00 00 00 00 00 00 00 00
      5     19295.400 DT     0000 Tx 8  00 00 00 00 00 00 00 00
      6     19500.600 DT     0000 Tx 8  00 00 00 00 00 00 00 00
      7     19705.200 DT     0000 Tx 8  00 00 00 00 00 00 00 00
      8     20592.700 DT 00000100 Tx 8  00 00 00 00 00 00 00 00
      9     20798.600 DT 00000100 Tx 8  00 00 00 00 00 00 00 00
     10     20956.000 DT 00000100 Tx 8  00 00 00 00 00 00 00 00
     11     21097.100 DT 00000100 Tx 8  00 00 00 00 00 00 00 00
"#;
        let data = TrcParser::new_from_text(trc, 0, false);
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.version, TrcVersion::V2_0);
        assert_eq!(data.packets.len(), 10);
        assert_eq!(data.packets[0].id, 0x100);
        assert!(data.packets[0].extended);
        assert_eq!(data.packets[4].id, 0x0);
        assert!(!data.packets[4].extended);
    }

    #[test]
    fn version_2_1() {
        let trc = r#"
;$FILEVERSION=2.1
;$STARTTIME=44548.6028595139
;$COLUMNS=N,O,T,B,I,d,R,L,D
;
;   C:\Users\User\Desktop\python-can\test\data\test_CanMessage_V2_1.trc
;   Start time: 18.12.2021 14:28:07.062.001
;   Generated by PEAK-Converter Version 2.2.4.136
;   Data imported from C:\Users\User\Desktop\python-can\test\data\test_CanMessage_V1_1.trc
;-------------------------------------------------------------------------------
;   Bus   Name            Connection               Protocol
;   N/A   N/A             N/A                      N/A      
;-------------------------------------------------------------------------------
;   Message   Time    Type    ID     Rx/Tx
;   Number    Offset  |  Bus  [hex]  |  Reserved
;   |         [ms]    |  |    |      |  |  Data Length Code
;   |         |       |  |    |      |  |  |    Data [hex] ...
;   |         |       |  |    |      |  |  |    | 
;---+-- ------+------ +- +- --+----- +- +- +--- +- -- -- -- -- -- -- --
      1     17535.400 DT 1  00000201 Tx -  8    02 00 01 00 00 00 00 00
      2     17540.300 ST 1         - Rx -  4    00 00 00 08
      3     17700.300 DT 1  00000100 Tx -  8    00 00 00 00 00 00 00 00
      4     17873.800 DT 1  00000100 Tx -  8    00 00 00 00 00 00 00 00
      5     19295.400 DT 1      0000 Tx -  8    00 00 00 00 00 00 00 00
      6     19500.600 DT 1      0000 Tx -  8    00 00 00 00 00 00 00 00
      7     19705.200 DT 1      0000 Tx -  8    00 00 00 00 00 00 00 00
      8     20592.700 DT 1  00000100 Tx -  8    00 00 00 00 00 00 00 00
      9     20798.600 DT 1  00000100 Tx -  8    00 00 00 00 00 00 00 00
     10     20956.000 DT 1  00000100 Tx -  8    00 00 00 00 00 00 00 00
     11     21097.100 DT 1  00000100 Tx -  8    00 00 00 00 00 00 00 FF
"#;
        let data = TrcParser::new_from_text(trc, 0, false);
        assert!(data.is_ok());
        let data = data.unwrap();
        assert_eq!(data.version, TrcVersion::V2_1);
        assert_eq!(data.packets.len(), 10);
        assert_eq!(data.packets[0].id, 0x201);
        assert!(data.packets[0].extended);
        assert_eq!(data.packets[4].id, 0x0);
        assert!(!data.packets[4].extended);
        assert_eq!(data.packets[9].bytes[7], 0xff);
    }
}
