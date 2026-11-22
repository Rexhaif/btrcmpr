# btrcmpr

A parallel lossless compressor written in Rust for the Raspberry Pi 5. It has 22 presets and no crate dependencies.

```sh
cargo build --release --offline --locked --bin btrcmpr
target/release/btrcmpr c -3 input.bin archive.btr 4
target/release/btrcmpr d archive.btr restored.bin 4
```

The default is level 3 with one worker per available core. The CLI reads the whole file into memory. The build targets the CPU that compiles it.

The encoder uses literals and backward matches for ordinary bytes, predicts neighboring 16-bit samples, groups similar sections in object archives, and learns phrase dictionaries for large text at high presets. Archives store their own models. See [presets](PRESETS.md) and [codec design](docs/design-notes.md).

## Benchmarks

Raspberry Pi 5, four workers, zstd 1.5.7. Each zstd worker writes a separate frame. A preset passes on a file when btrcmpr produces a smaller archive, encodes faster, and decodes at least 95% as fast as zstd.

| Release benchmark | Presets | Passed | Also decoded faster |
|---|---|---:|---:|
| [Three tuning files: enwik8, x-ray, Mozilla](bench/results/presets/native-combined-1/release.md) | 1–22 | 66/66 | 63 |
| [Other 25 Squash files](bench/results/squash-extra-20261122/outcomes.md) | 1, 3, 10, 22 | 4/100 | 3 |

The [subsequent build](bench/results/compatibility/README.md) reproduced all 166 archives and passed four of six timing checks. Its level-1 enwik8 decode reached 92.5% of zstd throughput.

[Run the benchmarks](bench/README.md) · [Results](bench/results/README.md)
