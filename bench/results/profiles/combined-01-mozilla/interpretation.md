# Why the level-one Mozilla race is close

The [frozen native counters](README.md) measured 23.07 vs 23.86 retired instructions per byte on encode. Rust executed 3.3% fewer instructions and consumed 3.7% more summed CPU cycles per byte in this run. Its [cycle sample](encode-cycles.txt) assigns 56.8% of Rust samples to the inlined object LZ routine; the 7.1% at the four-lane byte histogram is another optimization opportunity. Zstd spent 59.9% of its samples in its fast match finder. At this level both encoders use a short search. There is no twofold encode instruction advantage here.

On decode Rust executes 4.14 vs 8.71 instructions per byte, but spends 4.85 vs 4.41 CPU cycles. Lower instruction counts alone do not predict end-to-end throughput. Commands are unpacked in groups on ARM, and fixed-width records can be scattered straight into their original positions. The [actual block mix](../../presets/integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/representations.md) shows both representations in this file. These counters describe the **whole file**; they do not isolate each representation's contribution.

The [paired speed measurements](../../../../docs/design-notes.md) show how a small encode advantage changes between runs. Sampling and counter runs are separate from those acceptance timings.
