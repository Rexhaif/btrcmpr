# Compression presets

The level sets the encoder's search budget. Ordinary bytes use backward matches; smooth 16-bit data uses a predictor; recognized object archives can use separate encodings for instructions and records. From level 13, large text jobs can learn a phrase dictionary. The decoder reads the model from the archive.

Settings below are for four ARM workers. Object window sizes are target job sizes in KiB, in the order ordinary sections / records / instructions. A single size applies to all three.

| Level | Large text (as in enwik8) | Smooth 16-bit data (as in x-ray) | Object archive window, KiB |
|---:|---|---|---:|
| 1 | Two-position match scanner; reuse literal and distance models learned from a sample of each region. | Adjacent-sample prediction. | 192 / 128 / 192 |
| 2 | Paired six-byte lookups and 896 KiB jobs. | Search up to 2,048 samples back; 512 KiB jobs. | 128 |
| 3 | Short and long prefix indices, each with 65,536 buckets. | Search up to 4,096 samples back; 1,024 KiB jobs. | 128 |
| 4 | Double both prefix indices to 131,072 buckets. | Same as 3. | 128 |
| 5 | Long-distance positions, with a 4 MiB job cap. | Search up to 2,048 samples back; 2,048 KiB jobs reduce repeated row seeds. | 160 |
| 6 | Price matches by distance and sample the longer prefix index; 5 MiB cap. | Same as 5. | 112 |
| 7 | Wider match indices, sampled twelve-byte prefixes and a 12 MiB cap. | Same as 5. | 128 |
| 8 | Price short distant matches more carefully; use the twelve-byte index on one quarter of short-prefix buckets. | Same as 5. | 144 |
| 9 | Increase twelve-byte coverage to half the buckets. Encode command metadata in pages sharing one match history. | Same as 5. | 152 |
| 10 | Index all twelve-byte prefixes and use a larger short-prefix table. | Same as 5. | 160 |
| 11 | Retain another previous short-prefix match and entropy-code excess lengths. | Same as 5. | 168 |
| 12 | Add a nine-byte index for matches missed by the seven- and twelve-byte indices. | Search up to 8,192 samples back with a larger shortlist; 2,048 KiB jobs. | 176 |
| 13 | Each worker learns up to 16,384 adjacent-token merges; phrases up to 48 bytes. | Same as 12. | 184 |
| 14 | Raise the local merge budget to 32,768 and phrase width to 64 bytes. | Same as 12. | 192 |
| 15 | Raise the local merge budget to 65,536. | Same as 12. | 200 |
| 16 | Learn locally as at 15; share exact phrase spellings in groups of two jobs. | Same as 12. | 272 |
| 17 | Share one dictionary among four jobs; each still decodes its own token stream. | Same as 12. | 512 / 128 / 352 |
| 18 | Allow another 65,536 phrase merges across the four local streams. | Same as 12. | 640 |
| 19 | Use 32,768 local and 131,072 pooled merges; admit pairs seen twice and condition the token-group tags. | Same as 12. | 656 |
| 20 | Compress dictionary fields as well as tokens; use unconditional token-group tags. | Same as 12. | 672 / 672 / 656 |
| 21 | Keep the compressed dictionary fields and condition the token-group tags. | Same as 12. | 704 |
| 22 | Also try context coding for dictionary byte fields and keep the smaller representation. | Same as 12. | 448 |

From level 4 the encoder separates instruction sections. From level 5 it can store compressible record columns separately. Level 20 keeps the level-19 instruction window: a larger window increased the instruction payload. On x-ray, several levels chose the same row stride, producing five distinct archive sizes across 22 levels.

`python3 scripts/frame_info.py FILE.btr` lists stored block types, payload sizes, framing bytes and row strides. Shared dictionary bytes count toward the first block's payload.

[Benchmark rates](bench/results/presets/integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/rates.md) · [Stored block counts](bench/results/presets/integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/representations.md)

Source: [dispatch](src/lib.rs), [prefix searches](src/lz.rs), [local phrases](src/grammar/worddag.rs), [shared phrases](src/grammar/worddag/shared.rs), [dictionary packing](src/grammar/worddag/bookpack.rs), [signal predictor](src/signalgrid.rs), [object windows](src/layout.rs).
