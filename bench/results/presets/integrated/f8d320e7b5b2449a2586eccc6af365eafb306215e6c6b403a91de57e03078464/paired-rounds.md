# Variation between paired rounds

The [preset acceptance table](preset-ladder.md) compares each codec’s median time. This table instead compares Rust and zstd within each alternating round: Rust must encode faster and decode at least 95% as fast. Counting individual wins helps identify comparisons that are sensitive to timing. Five pairs are too few to establish a guaranteed margin. No rows are discarded.

In 63 of 66 verified file/level comparisons, Rust encoded faster in every pair; in 60, every pair met the decode allowance. [Full data, including minima, maxima and paired medians](paired-rounds.csv).

The six cells with the fewest rounds passing both speed thresholds, ordered by number passing, then paired encode median. Each speed is Rust throughput divided by zstd throughput within one round.

| File / level | Encode wins | Decodes within 5% | Both in same round | Median paired encode | Median paired decode |
|---|---:|---:|---:|---:|---:|
| [mozilla / 1](../../native-combined-1/level-01-mozilla.csv) | 3/5 | 4/5 | 3/5 | 1.015× | 1.003× |
| [enwik8 / 1](../../native-combined-1/level-01-enwik8.csv) | 4/5 | 3/5 | 3/5 | 1.040× | 0.972× |
| [enwik8 / 9](l09/level-09-enwik8.csv) | 5/5 | 3/5 | 3/5 | 1.085× | 0.969× |
| [enwik8 / 8](l08/level-08-enwik8.csv) | 4/5 | 5/5 | 4/5 | 1.024× | 0.993× |
| [mozilla / 18](l18/level-18-mozilla.csv) | 5/5 | 4/5 | 4/5 | 3.279× | 1.006× |
| [mozilla / 20](l20/level-20-mozilla.csv) | 5/5 | 4/5 | 4/5 | 3.682× | 1.010× |
