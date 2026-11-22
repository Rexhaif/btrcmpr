# Archive compatibility

The rebuilt CLI reproduced all 166 release archives byte for byte and restored their original contents: 22 presets on the three tuning files and four presets on the 25 other Squash files. It also read [three older archives](early-archives.json). The Rust tests cover a [133-byte phrase block](../../../tests/fixtures/legacy-phrases.block) with tag 43, mode 1.

[Archive hashes](archives.csv) · [Build and source hashes](verification.json)

## Timing checks

The rebuilt binary passed four of six checks. Rates are relative to zstd: a pass requires a smaller archive, faster encode and decode at or above 0.95×. Each row compares the original release with the rebuild.

| Input | Level | Bytes saved vs zstd | Encode: release → rebuild | Decode: release → rebuild | Rebuild result |
|---|---:|---:|---:|---:|---|
| enwik8 | 1 | +156,498 | 1.033× → 1.018× | 0.975× → 0.925× | Miss |
| x-ray | 1 | +2,442,046 | 3.624× → 4.840× | 2.015× → 2.229× | Pass |
| mozilla | 1 | +31,877 | 1.018× → 1.020× | 1.004× → 0.971× | Pass |
| enwik8 | 10 | +173,315 | 1.285× → 1.305× | 1.019× → 1.060× | Pass |
| grammar.lsp | 3 | -513 | 0.201× → 0.186× | 1.122× → 1.122× | Miss |
| osdb | 3 | +70,910 | 1.492× → 1.402× | 1.047× → 1.041× | Pass |

In a [same-session rerun](frozen-control.json), the release decoded enwik8 at level 1 at 1,618 MB/s (98.2% of zstd); the rebuild reached 1,513 MB/s (92.5%).

[Rates](spot-checks.csv) · [Paired timings](paired/) · [Executables and settings](spot-checks.json)
