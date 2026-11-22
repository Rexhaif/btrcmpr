# Cleanup verification

The cleaned CLI reproduced **166 of 166 archives byte for byte** and restored their original contents. This covers all 22 presets on the original three inputs and four presets on each of the other 25 Squash files, using four workers. Each replay checked the historical archive length and SHA-256, then checked the restored file against the published input SHA-256. The [per-file checks](archives.csv) and [build and source identities](verification.json) identify exactly what was verified.

The same CLI also restored [three earlier archives](early-archives.json). The Rust test suite reads and reproduces a [133-byte phrase block](../../../tests/fixtures/legacy-phrases.block) from the pre-cleanup source (tag 43, mode 1).

## Timing spot-checks

The freshly profiled build met all three requirements on **4 of six** cases selected before timing. The gate requires a smaller archive and faster encoding, with decode throughput at least 95% of zstd. The table compares btrcmpr throughput to zstd; above 1.00× is faster. The frozen release and the clean build were measured in separate sessions. These six checks do not update the verdict for the full timing matrix.

| Input | Level | Bytes saved vs zstd | Encode: frozen → clean | Decode: frozen → clean | Clean result |
|---|---:|---:|---:|---:|---|
| enwik8 | 1 | +156,498 | 1.033× → 1.018× | 0.975× → 0.925× | Miss |
| x-ray | 1 | +2,442,046 | 3.624× → 4.840× | 2.015× → 2.229× | Pass |
| mozilla | 1 | +31,877 | 1.018× → 1.020× | 1.004× → 0.971× | Pass |
| enwik8 | 10 | +173,315 | 1.285× → 1.305× | 1.019× → 1.060× | Pass |
| grammar.lsp | 3 | -513 | 0.201× → 0.186× | 1.122× → 1.122× | Miss |
| osdb | 3 | +70,910 | 1.492× → 1.402× | 1.047× → 1.041× | Pass |

The level-1 enwik8 decode miss also showed up in a same-session comparison: the [frozen binary rerun](frozen-control.json) decoded at 1,618 MB/s (98.2% of its zstd baseline), versus 1,513 MB/s for the clean build (92.5%). That is a 6.5% drop in observed decode throughput between the two builds. Both produced the same bytes. This check does not isolate which source or compiler change caused the difference. The other miss, `grammar.lsp`, was larger and slower to encode than zstd on both builds.

Four workers per codec; zstd 1.5.7 used one independent frame per worker. There was one warmup followed by five alternating measured pairs, with seven decodes per observation. The [absolute rates and trial hashes](spot-checks.csv), [raw timings](paired/) and [executable identities](spot-checks.json) are available.
