# What changes from level 1 to level 22

The level is an encoding budget. The codec examines the input: ordinary bytes use previous matches, a smooth 16-bit signal can use a predictor, and a tar archive containing recognized object sections can use different representations for code and records. Text has a fourth option at high levels: learn a dictionary of phrases from the input itself. All models, predictor choices, layout information and tokens needed to restore the file are included in its archive. Decoding requires neither the level nor a trained dictionary.

These are the defaults of the four-worker ARM codec. In the final column, window sizes are in KiB and apply to ordinary sections, records, and instructions, respectively; a single number applies to all three. These are job-size targets, not minimum file sizes.

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

Object window size is only one of its settings. From level 4 the codec separates instruction sections; from level 5, sufficiently compressible record columns can be stored separately. Level 20 retains the level-19 instruction window because increasing it made the measured instruction payload larger. Likewise, a longer signal search can choose the same stride; on x-ray there are five different archive sizes across the 22 settings. A level changes the work the encoder can do, not a guaranteed number of bytes for arbitrary input.

For an existing archive, `python3 scripts/frame_info.py FILE.btr` reports the actual stored block representations, their payload sizes and outer framing overhead. See the [block counts from available verified archives](bench/results/presets/integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/representations.md). Each job that uses the learned row predictor contributes to the stride histogram. Shared phrase-dictionary bytes are part of the leader block's payload and must not be added again.

[Measured ratios and encode/decode throughput at each setting](bench/results/presets/integrated/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464/rates.md). Implementation: [dispatch](src/lib.rs), [prefix searches](src/lz.rs), [local phrases](src/grammar/worddag.rs), [shared phrases](src/grammar/worddag/shared.rs), [dictionary packing](src/grammar/worddag/bookpack.rs), [signal predictor](src/signalgrid.rs), [object windows](src/layout.rs).
