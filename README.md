# btrcmpr

A lossless file compressor written in Rust for the Raspberry Pi 5’s four Cortex-A76 cores. It has 22 presets, uses multiple workers for both compression and decompression, and has no crate dependencies. The standalone program does not link zstd. Archives contain their own models; restoring a file does not require its preset, the original corpus, or the original worker count.

```sh
cargo build --release --offline --locked --bin btrcmpr
target/release/btrcmpr c -3 input.bin archive.btr 4
target/release/btrcmpr d archive.btr restored.bin 4
cmp input.bin restored.bin
```

Omit the preset to use level 3, or the worker count to use available parallelism. The CLI holds the file in memory. The checkout builds for the current CPU; compile on the target machine.

For ordinary data the encoder writes literals and backward matches. Its prefix indexes screen candidates before reading their text. Other paths exploit structure found in the input: predicting nearby 16-bit samples, grouping similar sections in supported object archives, and learning a phrase dictionary from large text. The ARM decoder unpacks commands in batches. The [preset guide](PRESETS.md) explains when these choices change; `python3 scripts/frame_info.py archive.btr` shows what a particular file used.

## Measurements

The measured release was tuned on enwik8, x-ray and Mozilla. These are **per-file** comparisons against zstd 1.5.7: the Rust archive must be smaller, compression faster, and decompression at least 95% as fast. Both codecs use four workers in each direction. The baseline uses four independent zstd frames; the numbers are not for a single `zstd -T4` stream.

| Frozen release on this Pi | Presets | Comparisons meeting all three requirements |
|---|---|---:|
| [Original three inputs](bench/results/presets/native-combined-1/release.md) | 1–22 | 66/66; 63 also decoded strictly faster |
| [Other 25 Squash inputs](bench/results/squash-extra-20261122/outcomes.md) | 1, 3, 10, 22 | 4/100; 3 also decoded strictly faster |

The second result limits how broadly the first should be applied. The cleaned build reproduced all 166 archives byte for byte. It passed four of six [timing spot-checks](bench/results/compatibility/README.md); level-1 enwik8 decode fell below the 95% threshold. See the [benchmark guide](bench/README.md) to measure a new build, the [development summary](docs/design-notes.md) for the decisions behind the codec, and the [results index](bench/results/README.md) for the raw evidence.
