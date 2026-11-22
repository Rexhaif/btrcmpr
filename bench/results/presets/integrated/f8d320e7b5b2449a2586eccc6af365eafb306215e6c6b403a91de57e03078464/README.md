# Integrated production matrix

**66 of 66 verified cells meet the 5% decode allowance and beat zstd on size and encode; 63 win strictly on all three.** All 22 levels have been verified on each of the three files. All rows use the same Rust production source snapshot, frozen comparator `f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464`, and zstd library `1279c4edfbfb08878b5b7fb40d392c1bdc9b30100d9b7a019d4aefaf10d2046c`. See [every measured size, ratio, speed, archive hash and raw pair](verified-cells.csv).

The [22-level table for this exact executable](preset-ladder.md) separates measured cells from missing ones and identifies the tightest margins. Each cell uses four workers per codec in both directions, five alternating pairs after warmup and seven decodes per time measurement. A production CLI without libzstd separately reproduces the reported size and restores the complete original. The [frozen manifest](../../native-combined-1/release.json) records the comparison and hashes.

| File | Cells verified | Pass at 5% | Verified levels missing a requirement |
|---|---:|---:|---|
| enwik8 | 22 | 22 | None in verified cells |
| x-ray | 22 | 22 | None in verified cells |
| mozilla | 22 | 22 | None in verified cells |

| Allowed decode throughput loss per cell | Verified cells meeting all requirements |
|---:|---:|
| 0% | 63 / 66 |
| 2% | 64 / 66 |
| 5% | 66 / 66 |
| 10% | 66 / 66 |

The [release comparison](../../native-combined-1/release.md) contains the closest margins.
