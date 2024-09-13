# CANDOR

CAN debugging library and tools.

Provides similar functionality to the [can-utils](https://github.com/linux-can/can-utils)
programs `cansniffer`, `candump`, `canbusload`, etc. with a terminal user interface
and associated Rust libraries.
```
CANdor 0.1.0                                                                            (? for help, q to quit) 
┌ Messages (<, > bus order) ──────────────────────────────────────┐┌ can0 @ 125000bps ─────────────────────────┐
│  can0          493       064  @ 95ms    04 00 00 00 00 00 00 00 ││██             5% (151 pps)                │
│  can0          493       065  @ 95ms    00 00 00 7f 1a 06       ││6997 packets                               │
│  can0          493       067  @ 95ms    4a 01 00 00 00 00       ││                                           │
│  can0          493       0FB  @ 95ms    02                      │└───────────────────────────────────────────┘
│  can0          493       0FA  @ 95ms    00 00 00 00 00 00 00 00 │┌ can1 @ 125000bps ─────────────────────────┐
│  can0          493       068  @ 95ms    00 00 00 00 00 00 00 00 ││██             5% (152 pps)                │
│  can0          493       069  @ 95ms    00 00 00 00 00 00 00 00 ││6997 packets                               │
│  can0          493       06A  @ 95ms    00 1e 00 1e 00 1e 00 00 ││                                           │
│  can0          493       06B  @ 95ms    00 00 00 00             │└───────────────────────────────────────────┘
│  can0          493       06C  @ 95ms    00 00 00 00             │┌ vcan0 @ 15000bps ─────────────────────────┐
│  can0          493       06D  @ 95ms    00 00 00 00             ││██████████     24% (49 pps)                │
│  can0          493       06E  @ 95ms    00 00 00 00             ││2337 packets                               │
│  can0          493       06F  @ 95ms    00 00 00 00             ││                                           │
│  can0          493       070  @ 95ms    00 00 00 00             │└───────────────────────────────────────────┘
│  can0           95       099  @ 495ms   00 00 00 00 00 00 00 00 │┌ vcan1 @ 15000bps ─────────────────────────┐
│  vcan1         936  00001201  @ 50ms    ce a4 05 00 00 00 00 00 ││████           10% (20 pps)                │
│  vcan0        1868       333  @ 25ms    47 09 02 52 3f ea b9 67 ││936 packets                                │
│  vcan0         469  00000889  @ 100ms   d1 32                   ││                                           │
│  can1          493       064  @ 95ms    04 00 00 00 00 00 00 00 │└───────────────────────────────────────────┘
│  can1          493       065  @ 95ms    00 00 00 7f 1a 06       │┌ Dump (A=adapter, D=DLC) ──────────────────┐
│  can1          493       067  @ 95ms    4a 01 00 00 00 00       ││vcan0        333  47 09 02 52 3f ea b9 67  │
│  can1          493       0FB  @ 95ms    02                      ││vcan0        889  d1 32                    │
│  can1          493       0FA  @ 95ms    00 00 00 00 00 00 00 00 ││vcan1       1201  ce a4 05 00 00 00 00 00  │
│  can1          493       068  @ 95ms    00 00 00 00 00 00 00 00 ││vcan0        333  29 c5 58 1c a5 5b 4b 13  │
│  can1          493       069  @ 95ms    00 00 00 00 00 00 00 00 ││vcan0        333  e4 af 41 71 31 57 5e 0d  │
│  can1          493       06A  @ 95ms    00 1e 00 1e 00 1e 00 00 ││can1          70  00 00 00 00              │
│  can1          493       06B  @ 95ms    00 00 00 00             ││can1          6F  00 00 00 00              │
│  can1          493       06C  @ 95ms    00 00 00 00             ││can1          6E  00 00 00 00              │
└─────────────────────────────────────────────────────────────────┘└───────────────────────────────────────────┘
```

## Features

- [x] Monitor multiple CAN interfaces
- [x] Show frequency, count, etc. grouped by ID
* [ ] Sorting / filtering the monitored data
- [ ] Decode CAN data using DBC files
- [ ] Analyze PCAP files with CAN traces
- [ ] Traffic generator, pattern- and DBC-based

## TODOs
* Clean up the stats, they are super basic right now
* Split the library and TUI into separate workspace projects
* Show live diffs in data

## License

[0-clause BSD license](LICENSE-0BSD.txt).
