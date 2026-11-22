# Running benchmarks

The comparator needs system libzstd. To build with LLVM profile data and run all 22 levels on enwik8, x-ray and Mozilla:

```sh
rustup component add llvm-tools-preview
python3 scripts/build-profiled.py --selected
scripts/fetch-data.sh
python3 scripts/preset-matrix.py --selected --out bench/local/my-run
```

To run levels 1, 3, 10 and 22 on the [other 25 Squash files](corpora/squash-extra.json):

```sh
scripts/fetch-data.sh --extra
python3 scripts/preset-matrix.py --selected --corpus extra --out bench/local/my-extra-run
```

Use a new output directory for each run. Add `--levels 1,3 --files x-ray` to select a subset. To finish an interrupted run with the same binaries and settings:

```sh
python3 scripts/preset-matrix.py --resume --out bench/local/my-run
```

## Measurements

Both codecs use four workers. Each zstd worker writes one frame so the decoder can process all four in parallel. Compressed sizes include framing. Throughput is uncompressed decimal MB per second, including worker startup and excluding disk I/O.

After one warmup, the runner times five pairs, alternating which codec runs first. It measures seven decodes per observation and reports median times. The standalone CLI checks the archive size and restores the input. Exit status 1 marks a missed target: the archive must be smaller than zstd's, encode faster, and decode at least 95% as fast.

## Hardware counters

```sh
BTR_LEVEL=10 scripts/perf-stat.sh --selected c ours bench/data/enwik8 4 3
BTR_LEVEL=10 scripts/perf-stat.sh --selected c zstd bench/data/enwik8 4 3
```

Use `d` for decode. The last two arguments are workers and repetitions. Set `PERF` to the perf executable if needed. Counters exclude warmup and file I/O.

[Published results](results/README.md)
