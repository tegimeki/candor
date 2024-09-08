# CANDOR

CAN debugging library and tools.

Provides similar functionality to the [can-utils](https://github.com/linux-can/can-utils)
programs `cansniffer`, `candump`, `canbusload`, etc. with a terminal user interface
and associated Rust libraries.
```
CANdor 0.1.0                                                                                             (? for help, q to quit) 
┌ Monitor ──────────────────────────────────────────────────────────────────┐┌ can0 @ 125000bps ────────────────────────────────┐
│  can0         3420       064  @ 95ms    04 00 00 00 00 00 00 00           ││███                5% (146 pps)                   │
│  can0         3420       065  @ 95ms    00 00 00 7f 1a 06                 ││48536 packets                                     │
│  can0         3420       067  @ 95ms    e8 03 00 00 00 00                 ││                                                  │
│  can0         3420       0FB  @ 95ms    02                                │└──────────────────────────────────────────────────┘
│→ can0         3420       0FA  @ 95ms    00 00 00 00 00 00 00 00           │┌ vcan0 @ 15000bps ────────────────────────────────┐
│  can0         3420       068  @ 95ms    00 00 00 00 00 00 00 00           ││█████████████      25% (50 pps)                   │
│  can0         3420       069  @ 95ms    00 00 00 00 00 00 00 00           ││16165 packets                                     │
│  can0         3420       06A  @ 95ms    00 1e 00 1e 00 1e 00 00           ││                                                  │
│  can0         3420       06B  @ 95ms    00 00 00 00                       │└──────────────────────────────────────────────────┘
│  can0         3420       06C  @ 95ms    00 00 00 00                       │┌ vcan1 @ 15000bps ────────────────────────────────┐
│  can0         3420       06D  @ 95ms    00 00 00 00                       ││████████           16% (40 pps)                   │
│  can0         3420       06E  @ 95ms    00 00 00 00                       ││12921 packets                                     │
│  can0         3420       06F  @ 95ms    00 00 00 00                       ││                                                  │
│  can0         3420       070  @ 95ms    00 00 00 00                       │└──────────────────────────────────────────────────┘
│  vcan1       12921  00001201  @ 25ms    5b 34 86 65 2a ca e7              │┌ Dump (A=adapter, D=DLC) ─────────────────────────┐
│  vcan0       12921       333  @ 25ms    e8 b7 31 3f 56 b6 2e 3e           ││vcan0        888   [8]   81 97 71 1b 79 77 2d 15  │
│  vcan0        3244  00000888  @ 100ms   81 97 71 1b 79 77 2d 15           ││vcan0        333   [8]   e8 b7 31 3f 56 b6 2e 3e  │
│  can0          656       099  @ 495ms   00 00 00 00 00 00 00 00           ││vcan1       1201   [7]   5b 34 86 65 2a ca e7     │
│                                                                           ││can0          70   [4]   00 00 00 00              │
│                                                                           ││can0          6F   [4]   00 00 00 00              │
│                                                                           ││can0          6E   [4]   00 00 00 00              │
│                                                                           ││can0          6D   [4]   00 00 00 00              │
│                                                                           ││can0          6C   [4]   00 00 00 00              │
│                                                                           ││can0          6B   [4]   00 00 00 00              │
│                                                                           ││can0          6A   [8]   00 1e 00 1e 00 1e 00 00  │
│                                                                           ││can0          69   [8]   00 00 00 00 00 00 00 00  │
│                                                                           ││can0          68   [8]   00 00 00 00 00 00 00 00  │
│                                                                           ││can0          FA   [8]   00 00 00 00 00 00 00 00  │
│                                                                           ││can0          FB   [1]   02                       │
└───────────────────────────────────────────────────────────────────────────┘└──────────────────────────────────────────────────┘
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
