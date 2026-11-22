# Decode thresholds across 22 presets

Counts cover all 22 presets on enwik8, x-ray and Mozilla. Each pass requires a smaller archive and faster encode than zstd. The table varies the allowed loss in decode throughput.

| Allowed decode loss | File/preset pairs passing |
|---:|---:|
| 0% | 63 / 66 |
| 2% | 64 / 66 |
| 5% | 66 / 66 |
| 10% | 66 / 66 |

[Per-file margins](preset-ladder.md) · [Release results](../../native-combined-1/release.md) · [Rates and archive hashes](verified-cells.csv)
