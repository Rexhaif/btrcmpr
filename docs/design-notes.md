# Codec design

**Match search.** Level 1 reads two overlapping prefixes from one word and reuses literal and distance models across neighboring blocks. The [six-byte matcher](../bench/results/presets/level-one-candidates.md) produced a smaller enwik8 archive but encoded more slowly than zstd. Higher presets index several prefix lengths and the last match distance. Fingerprints discard candidates before the encoder reads their text.

**ARM decoding.** Separate streams for tags, literals, lengths and distances let the decoder unpack commands in batches. On [Mozilla at level 1](../bench/results/profiles/combined-01-mozilla/README.md), btrcmpr decoded in 4.14 instructions per byte against zstd's 8.71, but used 4.85 CPU cycles per byte against 4.41. On [enwik8 at level 10](../bench/results/profiles/enwik8-general/combined-10/interpretation.md), btrcmpr's encoder used 93.26 instructions per byte against 295.96; its sparse probes retired fewer instructions per cycle.

**Text.** Workers merge frequent pairs of adjacent tokens. Higher levels share dictionary entries across workers, pool pair counts and compress the dictionary fields. Each job retains a separate token stream.

**Object archives.** The encoder checks tar headers, groups recognized object sections, and records the layout needed to put each byte back. Instruction sections, fixed-width records and ordinary bytes have separate match windows at some presets. The decoder writes record fields directly to their original positions.

**Signals.** The encoder predicts from adjacent 16-bit samples or estimates a row stride. Larger jobs store fewer row seeds; longer stride searches can find earlier rows. The [preset table](../PRESETS.md) lists search distances and job sizes.

**Compiler.** Release benchmarks used thin LTO and LLVM profile data trained on separate inputs. The ARM build uses the two-position scanner with or without profile data.

## Archived experiments

The [source snapshot](../bench/archive/development-snapshot.tar.gz) contains the earlier encoders, experiments and measurements. Read its index without unpacking:

```sh
tar -xOzf bench/archive/development-snapshot.tar.gz bench/results/presets/research/README.md
```
