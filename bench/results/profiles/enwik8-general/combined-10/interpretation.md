# Where the level-10 encoder spends its instructions

The [counter run](README.md) on the frozen combined release measured **93.26 vs 295.96 retired instructions per input byte**. Rust used 68.5% fewer instructions (3.17× as many on zstd). The CPU-cycle ratio was 1.30×. Both used four workers and produced 30,566,906 and 30,740,221 bytes respectively.

In the independent paired wall-time benchmark Rust encoded at **14.58 vs 11.34 MB/s** and decoded at **721.39 vs 708.25 MB/s**. The encoder retired 0.175 vs 0.427 instructions per CPU cycle and 14.78 vs 39.70 branches per input byte. Cutting the number of instructions did not cut time by the same factor: the sparse Rust probes are expensive operations to retire.

| Encoder | Hottest sampled function | Instruction samples | Estimated instructions per input byte there |
|---|---|---:|---:|
| ours | `btrcmpr::lz::middle_text` | 86.17% | 80.4 |
| zstd | `ZSTD_RowFindBestMatch_noDict_5_5.constprop.0` | 92.55% | 273.9 |

The [instruction samples](encode-instructions.txt) and hardware totals come from separate runs of the same native profile driver. Multiplying a sample share by an instruction total gives an estimate, not an exact function counter; the Rust and zstd function boundaries also differ.

The earlier [instrumented search study](../../../../../docs/design-notes.md) explains the difference. Zstd's row matcher checks candidates surviving a fingerprint mask; it performed **573,728,700** preliminary candidate checks on enwik8, of which **47,972,622** reached the full length comparison. The instrumented Rust parser made **57,954,544** candidate-text comparisons including lookahead and repeats. Rust probes a bounded number of positions indexed by several prefix lengths and the last distance; the fingerprint in each table entry can reject a candidate before reading its text. Those two counts are different units of work, not a tenfold instruction-speed prediction.

A [controlled change to zstd](../../../../../docs/design-notes.md) reduced its searchLog from 5 to 3 with the rest of level 10 fixed: instructions fell from 295.96 to 178.19 per byte, and its archive grew by 625,934 bytes. This shows the cost of search depth within zstd. The unmodified level-10 reference is used for the release comparison. These results apply to this level and input; the Mozilla level-one encoders retire [nearly the same instruction count](../../combined-01-mozilla/interpretation.md).
