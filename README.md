# CANDOR

A tool for CAN-bus debugging, observation, and reverse-engineering.

Provides similar functionality to the [can-utils](https://github.com/linux-can/can-utils)
programs `cansniffer`, `candump`, `canbusload`, etc. with a terminal user interface.

```
CANdor 0.1.0                                                                                   (? for help, q to quit)
┌ Messages (<, > = bus order; W, w = width, u = show/hide undecoded) ────┐┌ vcan0 @ 15000bps ─────────────────────────┐
│BMS_info                 100ms      0a 02 aa 55 34 12 0b 00             ││███████        16% (30 pps)                │
│     300                              BMS_infoIndex 10                  ││624 packets                                │
│                                                                        ││                                           │
│BMS_SOC                  100ms      32 ec 87 7e 7c e8 27 02             │└───────────────────────────────────────────┘
│     292                              BOL_energy 100.000kW              │┌ vcan1 @ 15000bps ─────────────────────────┐
│                                      SOC_max 100.000%                  ││███             7% (18 pps)                │
│                                      SOC_ave 49.700%                   ││395 packets                                │
│                                      SOC_UI 50.700%                    ││                                           │
│                                      SOC_min 5.000%                    │└───────────────────────────────────────────┘
│                                      BMS_battTempPct 54.800%           │┌ Dump  (A=adapter, D=DLC) ─────────────────┐
│                                                                        ││vcan1        321  1f 3f e0 04 6d a6 e1 6a  │
│BMS_contactorRequest     100ms      c9 00 ce 04 01 00 00 00             ││vcan0        232  c9 00 ce 04 01 00 00 00  │
│     232                              BMS_ensShouldBeActiveForDrive 1   ││vcan0        292  32 ec 87 7e 7c e8 27 02  │
│                                      BMS_fcContactorRequest 1          ││vcan0        300  0a 02 aa 55 34 12 0b 00  │
│                                      BMS_fcLinkOkToEnergizeRequest 1   ││vcan1        352  b3 fb b7 5a 19 8e 33 77  │
│                                      BMS_gpoHasCompleted 1             ││vcan1        392  70 ff 77 48              │
│                                      BMS_internalHvilSenseV 1.230V     ││vcan1        321  e6 cd aa 70 f4 38 d1 35  │
│                                      BMS_packContactorRequest 1        ││vcan0        232  c9 00 ce 04 01 00 00 00  │
│                                      BMS_pcsPwmDisable 0               ││vcan0        292  32 f0 87 be 7c e8 27 02  │
│                                                                        ││vcan0        300  0a 02 aa 55 34 12 0b 00  │
│     321                 100ms      1f 3f e0 04 6d a6 e1 6a             ││vcan1        321  d1 af 56 7e da 2c 59 71  │
│                                                                        ││vcan0        232  c9 00 ce 04 01 00 00 00  │
│     392                 200ms      70 ff 77 48                         ││vcan0        292  32 f4 87 fe 7c e8 27 02  │
│                                                                        ││vcan0        300  0a 02 aa 55 34 12 0b 00  │
│     352                 250ms      b3 fb b7 5a 19 8e 33 77             ││vcan1        392  fc ba 19 4a              │
│                                                                        ││vcan1        321  40 66 df 30 ca 0c ee 1c  │
└────────────────────────────────────────────────────────────────────────┘└───────────────────────────────────────────┘
```

## Features

- [x] Monitor multiple CAN interfaces
- [x] Show frequency, count, etc. grouped by ID
* [ ] Sorting / filtering the monitored data
- [x] Decode CAN data using DBC files (works, needs refining)
- [ ] Analyze PCAP files with CAN traces
- [ ] Traffic generator, pattern- and DBC-based

## License

[0-clause BSD license](LICENSE-0BSD.txt).
