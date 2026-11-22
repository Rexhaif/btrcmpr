# Benchmark results

| Corpus | Evidence |
|---|---|
| enwik8, x-ray and Mozilla, levels 1–22 | [Release comparison](presets/native-combined-1/release.md), [frozen build and trial hashes](presets/native-combined-1/release.json) |
| All other 25 Squash inputs, levels 1, 3, 10 and 22 | [Per-file outcomes](squash-extra-20261122/outcomes.md), [ratios and throughput](squash-extra-20261122/comparisons.csv), [round-trip verification](squash-extra-20261122/verification.json) |
| ARM performance counters from the same release | [enwik8 encode at level 10](profiles/enwik8-general/combined-10/interpretation.md), [Mozilla at level 1](profiles/combined-01-mozilla/README.md) |
| Cleaned build | [Archive compatibility and timing checks](compatibility/README.md) |

See the [benchmark guide](../README.md) for running a fresh comparison, and the [development summary](../../docs/design-notes.md) for earlier experiments.
