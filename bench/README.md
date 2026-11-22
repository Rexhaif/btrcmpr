# Building and benchmarking

The standalone codec needs Rust. The tools that compare it with zstd also need system libzstd. The normal release includes the selected ARM scanner; compiler training is optional.

```sh
# A profiled build with separate benchmark and perf programs.
rustup component add llvm-tools-preview
python3 scripts/build-profiled.py --selected

# Fetch the original three files, then run all 22 presets.
scripts/fetch-data.sh
python3 scripts/preset-matrix.py --selected --out bench/local/my-run

# An interrupted run resumes using its original frozen binaries and settings.
python3 scripts/preset-matrix.py --resume --out bench/local/my-run
```

For a shorter run, use a fresh output directory and add e.g. `--levels 1,3 --files x-ray`. Results include five alternating pairs after warmup and seven decodes per timing. The selected CLI separately compresses and restores each input. The script records hashes, ratios, absolute throughput and the 5% decode test. A benchmark can finish correctly and exit with status 1 if the requested performance target was missed.

To run the same protocol on the **other 25 Squash inputs**, use the shared downloader and matrix runner. The [input manifest](corpora/squash-extra.json) pins their download URLs, lengths and hashes. This corpus defaults to the four published presets (1, 3, 10 and 22). You can narrow it with the same `--levels` and `--files` options.

```sh
scripts/fetch-data.sh --extra
python3 scripts/preset-matrix.py --selected --corpus extra --out bench/local/my-extra-run
```

Both sides receive four workers. One zstd frame per quarter-file permits parallel decoding; the size includes all framing. Speeds use original decimal MB/s. In-memory timing includes worker startup but excludes file I/O. For one set of hardware counters, run:

```sh
BTR_LEVEL=10 scripts/perf-stat.sh --selected c ours bench/data/enwik8 4 3
BTR_LEVEL=10 scripts/perf-stat.sh --selected c zstd bench/data/enwik8 4 3
```

Use `d` to profile decoding and set `PERF` if perf is not on PATH. The counter region excludes file I/O and warmup. Timing jobs share a lock with selected perf runs. [Published runs](results/README.md) are frozen observations, independent of local rebuilds.
