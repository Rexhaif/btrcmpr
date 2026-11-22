# Level 10, enwik8: hardware counters on the combined build

Four Cortex-A76 workers on each side. The [paired wall-time trials](../../../presets/integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/l10/level-10-enwik8.csv) used a separate comparator from the same [native build](../../../presets/native-combined-1/codegen.json). Each counter run checked the paired archive size and original contents. Counters cover the measured operation only; each value is summed over all workers and divided by the number of uncompressed bytes processed.

| Operation | Event per input byte | Rust | zstd | zstd / Rust |
|---|---|---:|---:|---:|
| Encode | Instructions | 93.263 | 295.964 | 3.17× |
| Encode | CPU cycles | 533.716 | 693.358 | 1.30× |
| Encode | Branch misses | 0.629 | 1.104 | 1.75× |
| Encode | L1 data cache refills | 2.906 | 2.492 | 0.86× |
| Encode | L2 data cache refills | 2.916 | 5.490 | 1.88× |
| Decode | Instructions | 9.872 | 10.559 | 1.07× |
| Decode | CPU cycles | 10.727 | 11.807 | 1.10× |
| Decode | Branch misses | 0.071 | 0.013 | 0.19× |
| Decode | L1 data cache refills | 0.036 | 0.093 | 2.57× |
| Decode | L2 data cache refills | 0.061 | 0.052 | 0.84× |

Raw perf output and executable identities: [encode](c/manifest.json), [decode](d/manifest.json).
