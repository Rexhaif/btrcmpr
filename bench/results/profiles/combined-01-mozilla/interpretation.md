# Mozilla, level 1

Both encoders use a short match search. btrcmpr retired 23.07 instructions per input byte against zstd’s 23.86 and used 3.7% more CPU cycles. In the [cycle samples](encode-cycles.txt), btrcmpr spent 56.8% in its object LZ parser and 7.1% in its byte histogram. zstd spent 59.9% in its fast match finder.

Decoding took 4.14 instructions per output byte for btrcmpr and 8.71 for zstd, but 4.85 and 4.41 CPU cycles respectively. The [archive](../../presets/integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/representations.md) contains both batched LZ commands and fixed-width records. The [counters](README.md) cover the whole file.
