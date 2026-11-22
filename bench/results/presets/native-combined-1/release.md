# Release benchmarks: enwik8, x-ray and Mozilla

On a Raspberry Pi 5 with four workers, btrcmpr produced a smaller archive and encoded faster than zstd 1.5.7 in all 66 comparisons. It decoded faster in 63; the remaining three reached at least 95% of zstd throughput.

[All rates](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/rates.md) · [Paired rounds](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/paired-rounds.md) · [Binary and archive hashes](release.json) · [25 other Squash files](../../squash-extra-20261122/README.md)

The rate columns show btrcmpr throughput divided by zstd throughput. Parentheses give the preset.

| File | Levels checked | Distinct Rust sizes | Smallest saving | Lowest encode speedup | Lowest decode speedup |
|---|---:|---:|---:|---:|---:|
| enwik8 | 22 | 22 | 1,835 B (−6) | 1.025× (−8) | 0.969× (−9) |
| x-ray | 22 | 5 | 1,571,708 B (−19) | 2.736× (−2) | 1.289× (−2) |
| mozilla | 22 | 22 | 25,332 B (−3) | 1.018× (−1) | 1.004× (−1) |

Selected presets, btrcmpr / zstd. Ratio is input bytes divided by archive bytes; throughput is decimal MB/s of input.

| Input | Preset | Ratio | Encode MB/s | Decode MB/s |
|---|---:|---:|---:|---:|
| enwik8 | [−1](level-01-enwik8.csv) | 2.4683 / 2.4588 | 647.42 / 626.61 | 1566.64 / 1607.52 |
| x-ray | [−1](level-01-x-ray.csv) | 1.9569 / 1.2513 | 2520.97 / 695.55 | 2663.40 / 1322.06 |
| mozilla | [−1](level-01-mozilla.csv) | 2.5698 / 2.5657 | 775.09 / 761.15 | 1674.84 / 1668.44 |
| enwik8 | [−10](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/l10/level-10-enwik8.csv) | 3.2715 / 3.2531 | 14.58 / 11.34 | 721.39 / 708.25 |
| x-ray | [−10](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/tracks/x-ray/level-10-x-ray.csv) | 2.3003 / 1.5471 | 579.86 / 11.46 | 1478.22 / 531.59 |
| mozilla | [−10](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/l10/level-10-mozilla.csv) | 3.0908 / 3.0644 | 40.16 / 21.17 | 1360.94 / 1167.78 |
| enwik8 | [−22](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/l22/level-22-enwik8.csv) | 3.7327 / 3.7299 | 6.38 / 2.52 | 759.71 / 653.26 |
| x-ray | [−22](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/tracks/x-ray/level-22-x-ray.csv) | 2.3018 / 1.6131 | 315.06 / 5.52 | 1373.58 / 593.84 |
| mozilla | [−22](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/l22/level-22-mozilla.csv) | 3.3813 / 3.3630 | 25.58 / 5.56 | 1133.01 / 1087.09 |

In 15 further level-1 pairs, btrcmpr encoded faster in 12 on enwik8 and all 15 on Mozilla. The level-15 enwik8 results use a rerun after a second worker pool overlapped the first.

[Method](../../../README.md) · [Level-1 candidates](../level-one-candidates.md) · [enwik8 instruction profile](../../profiles/enwik8-general/combined-10/interpretation.md) · [Mozilla counters](../../profiles/combined-01-mozilla/README.md)
