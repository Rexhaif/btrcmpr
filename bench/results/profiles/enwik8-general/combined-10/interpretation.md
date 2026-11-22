# enwik8 match search, level 10

btrcmpr encoded at 14.58 MB/s against zstd’s 11.34 MB/s. It retired 93.26 instructions per input byte against 295.96 (68.5% fewer). Its sparse index lookups retired 0.175 instructions per cycle against zstd’s 0.427. The resulting throughput difference was 1.29×. [Hardware counters](README.md).

| Encoder | Hottest sampled function | Instruction samples | Estimated instructions per input byte there |
|---|---|---:|---:|
| btrcmpr | `btrcmpr::lz::middle_text` | 86.17% | 80.4 |
| zstd | `ZSTD_RowFindBestMatch_noDict_5_5.constprop.0` | 92.55% | 273.9 |

The last column multiplies each function’s [share of instruction samples](encode-instructions.txt) by the hardware total per byte.

The zstd row matcher made 573,728,700 preliminary candidate checks; 47,972,622 reached a full length comparison. btrcmpr compared candidate text 57,954,544 times, including lookahead and repeat distances. It indexes positions by several prefix lengths and tests fingerprints before loading candidate text.

Reducing zstd’s `searchLog` from 5 to 3 with the other level-10 settings fixed cut its instruction count to 178.19 per byte and increased its archive by 625,934 bytes.
