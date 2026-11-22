# Verified release: all 22 compression presets

**66/66** file/level comparisons verified with the same Rust executable and zstd 1.5.7 on four Cortex-A76 workers in both directions. The Rust archive was smaller, encoding was faster, and decoding reached at least 95% of zstd throughput in every listed comparison. In **63** it also decoded strictly faster. Each archive was independently reproduced and restored by the standalone Rust CLI.

[Standalone executable](../../../../target/verified/all-22-f8d320e7b5b2/btrcmpr) · [All 66 absolute comparisons](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/rates.md) · [Individual alternating rounds](../integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/paired-rounds.md) · [Binary and archive hashes](release.json)

The following are the tightest margins for each input. The smallest size margin, lowest encode speedup and lowest decode speedup can occur at different levels. A speedup is Rust throughput divided by matched zstd throughput.

| File | Levels checked | Distinct Rust sizes | Smallest saving | Lowest encode speedup | Lowest decode speedup |
|---|---:|---:|---:|---:|---:|
| enwik8 | 22 | 22 | 1,835 B (−6) | 1.025× (−8) | 0.969× (−9) |
| x-ray | 22 | 5 | 1,571,708 B (−19) | 2.736× (−2) | 1.289× (−2) |
| mozilla | 22 | 22 | 25,332 B (−3) | 1.018× (−1) | 1.004× (−1) |

A few matched levels, **Rust / zstd**. Ratio is original bytes divided by archive bytes; rates are original decimal MB/s.

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

The [preset guide](../../../../PRESETS.md) describes the algorithms for text, 16-bit signals and object sections. The baseline encodes four independent standard zstd frames, one per worker, so it can decode in parallel too. Every size includes framing and model metadata. Each timing comparison uses five alternating pairs after warmup and seven decodes per observation; this is a result about median throughput on this machine.

The selected format uses literal bytes and backward matches for ordinary data. At medium levels the encoder probes positions indexed by several prefix lengths; an inline fingerprint often rules out a position without loading its text. It stores lengths, distances and literals separately, which lets the ARM decoder unpack commands in batches. At high text levels it learns frequent phrases and stores their dictionary in the archive. On smooth 16-bit inputs it predicts neighboring samples; on recognized object archives it groups similar sections and packs integer records by column. These paths depend on input structure, not benchmark filenames.

On [enwik8 level 10](../../profiles/enwik8-general/combined-10/interpretation.md), the selected encoder retired 93.26 vs 295.96 instructions per byte, a 68.5% reduction. Instruction samples attributed about 274 instructions per byte to zstd’s row match finder and about 80 to Rust’s main text parser. A [controlled change to zstd](../../../../docs/design-notes.md) reduced search depth alone: zstd then used 178.19 instructions per byte but its archive grew by 625,934 bytes. Rust’s sparse lookups retire fewer instructions per cycle, so its throughput advantage was 1.29×. On [Mozilla level 1](../../profiles/combined-01-mozilla/interpretation.md), encode instruction counts were much closer (23.07 vs 23.86); the twofold reduction there was on decode (4.14 vs 8.71).

The [candidate table](../level-one-candidates.md) gives absolute speeds and compression ratios against a matched zstd run for each level-one encoder. These tables describe the frozen binary. Changes to the source are not new throughput measurements.

A longer check on this binary passed level 1 on enwik8 and Mozilla, winning 12/15 and 15/15 individual encode pairs respectively. A source rebuild with identical machine instructions lost 0.37% on Mozilla encoding over five pairs and won 2.22% over fifteen. The [development summary and original records](../../../../docs/design-notes.md) explain the narrow margins. The [separate 25-file run](../../squash-extra-20261122/README.md) measures the same release outside the three tuning inputs.
