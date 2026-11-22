# Level-1 candidates

Rates on enwik8 are Rust / matched four-worker zstd, in MB/s; ratios are original bytes divided by archive bytes. “Files passing” applies the 5% decode allowance on all three original files. The combined build is the measured release.

| Candidate | Ratio, Rust / zstd | Encode MB/s, Rust / zstd | Decode MB/s, Rust / zstd | Files passing |
|---|---:|---:|---:|---:|
| Six-byte lookup | 2.4732 / 2.4588 | 555.55 / 590.13 | 1631.23 / 1637.84 | 1/3 |
| Same lookup, profile-guided compilation | 2.4732 / 2.4588 | 570.09 / 591.94 | 1650.70 / 1614.99 | 2/3 |
| Regional models and two-position scanner | 2.4683 / 2.4588 | 650.95 / 585.30 | 1621.28 / 1563.85 | 3/3 |
| Combined build, same level-one archive | 2.4683 / 2.4588 | 647.42 / 626.61 | 1566.64 / 1607.52 | 3/3 |
| Combined build with reusable object buffers | 2.4683 / 2.4588 | 651.38 / 630.38 | 1535.04 / 1630.62 | 2/3 |

The reusable-buffer build improved Mozilla encoding, but its enwik8 decode was 5.86% below zstd. The combined build passed the same cell. The [measured release](native-combined-1/release.md) includes all presets. The [development summary and archive](../../../docs/design-notes.md) preserve the complete set of alternatives, repeated measurements and exact source.
