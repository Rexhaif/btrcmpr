# Hardware counters: Mozilla, level 1

Counts are summed across four Cortex-A76 workers and divided by uncompressed bytes.

| Operation | Event per input byte | Rust | zstd | zstd / Rust |
|---|---|---:|---:|---:|
| Encode | Instructions | 23.067 | 23.860 | 1.03× |
| Encode | CPU cycles | 11.105 | 10.708 | 0.96× |
| Encode | Branch misses | 0.086 | 0.071 | 0.83× |
| Encode | L1 data cache refills | 0.128 | 0.371 | 2.89× |
| Encode | L2 data cache refills | 0.023 | 0.040 | 1.71× |
| Decode | Instructions | 4.137 | 8.706 | 2.10× |
| Decode | CPU cycles | 4.846 | 4.409 | 0.91× |
| Decode | Branch misses | 0.043 | 0.032 | 0.73× |
| Decode | L1 data cache refills | 0.021 | 0.030 | 1.46× |
| Decode | L2 data cache refills | 0.007 | 0.005 | 0.73× |

[Encode counters](c/manifest.json) · [Decode counters](d/manifest.json) · [Paired timings](../../presets/native-combined-1/level-01-mozilla.csv) · [Build](../../presets/native-combined-1/codegen.json)
