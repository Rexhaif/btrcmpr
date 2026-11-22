# Variation across five pairs

Each pair compares btrcmpr and zstd in one round. Across 66 file/preset combinations, btrcmpr encoded faster in all five rounds for 63 combinations and decoded at least 95% as fast in all five for 60. These six had the fewest rounds meeting both thresholds. Rates are btrcmpr / zstd. [All rounds](paired-rounds.csv) · [Median results](preset-ladder.md)

| File / level | Encode wins | Decodes within 5% | Both in same round | Median paired encode | Median paired decode |
|---|---:|---:|---:|---:|---:|
| [mozilla / 1](../../native-combined-1/level-01-mozilla.csv) | 3/5 | 4/5 | 3/5 | 1.015× | 1.003× |
| [enwik8 / 1](../../native-combined-1/level-01-enwik8.csv) | 4/5 | 3/5 | 3/5 | 1.040× | 0.972× |
| [enwik8 / 9](l09/level-09-enwik8.csv) | 5/5 | 3/5 | 3/5 | 1.085× | 0.969× |
| [enwik8 / 8](l08/level-08-enwik8.csv) | 4/5 | 5/5 | 4/5 | 1.024× | 0.993× |
| [mozilla / 18](l18/level-18-mozilla.csv) | 5/5 | 4/5 | 4/5 | 3.279× | 1.006× |
| [mozilla / 20](l20/level-20-mozilla.csv) | 5/5 | 4/5 | 4/5 | 3.682× | 1.010× |
