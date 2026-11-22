# Per-preset margins against zstd

Each entry links to its five timed pairs: **bytes saved / relative encode throughput / relative decode throughput**. A pass requires positive savings, encode above 1.0× and decode at least 0.95×. [Absolute rates and archive hashes](verified-cells.csv).

| Level | enwik8 | x-ray | Mozilla |
|---:|---:|---:|---:|
| 1 | [+156,498 / 1.033 / 0.975](../../native-combined-1/level-01-enwik8.csv) | [+2,442,046 / 3.624 / 2.015](../../native-combined-1/level-01-x-ray.csv) | [+31,877 / 1.018 / 1.004](../../native-combined-1/level-01-mozilla.csv) |
| 2 | [+112,500 / 1.190 / 1.082](l02/level-02-enwik8.csv) | [+2,967,493 / 2.736 / 1.289](l02/level-02-x-ray.csv) | [+477,240 / 1.200 / 1.081](l02/level-02-mozilla.csv) |
| 3 | [+335,178 / 1.287 / 1.147](l03/level-03-enwik8.csv) | [+2,395,554 / 17.810 / 1.462](l03/level-03-x-ray.csv) | [+25,332 / 1.554 / 1.181](l03/level-03-mozilla.csv) |
| 4 | [+152,080 / 1.086 / 1.501](l04/level-04-enwik8.csv) | [+2,061,212 / 35.929 / 2.021](tracks/x-ray/level-04-x-ray.csv) | [+205,835 / 7.269 / 1.188](l04/level-04-mozilla.csv) |
| 5 | [+32,216 / 1.256 / 1.132](l05/level-05-enwik8.csv) | [+1,921,685 / 23.085 / 1.909](tracks/x-ray/level-05-x-ray.csv) | [+357,964 / 6.467 / 1.051](l05/level-05-mozilla.csv) |
| 6 | [+1,835 / 1.090 / 1.003](l06/level-06-enwik8.csv) | [+1,917,251 / 21.807 / 1.903](l06/level-06-x-ray.csv) | [+27,593 / 1.572 / 1.042](l06/level-06-mozilla.csv) |
| 7 | [+32,117 / 1.195 / 1.030](l07/level-07-enwik8.csv) | [+1,834,544 / 31.590 / 2.551](tracks/x-ray/level-07-x-ray.csv) | [+193,644 / 1.539 / 1.103](l07/level-07-mozilla.csv) |
| 8 | [+46,967 / 1.025 / 0.989](l08/level-08-enwik8.csv) | [+1,836,517 / 32.069 / 2.553](tracks/x-ray/level-08-x-ray.csv) | [+178,095 / 1.516 / 1.059](l08/level-08-mozilla.csv) |
| 9 | [+32,922 / 1.087 / 0.969](l09/level-09-enwik8.csv) | [+1,804,737 / 37.682 / 2.724](tracks/x-ray/level-09-x-ray.csv) | [+152,972 / 1.324 / 1.172](l09/level-09-mozilla.csv) |
| 10 | [+173,315 / 1.285 / 1.019](l10/level-10-enwik8.csv) | [+1,793,603 / 50.601 / 2.781](tracks/x-ray/level-10-x-ray.csv) | [+142,948 / 1.897 / 1.165](l10/level-10-mozilla.csv) |
| 11 | [+22,376 / 1.430 / 1.085](l11/level-11-enwik8.csv) | [+1,791,742 / 51.616 / 2.613](tracks/x-ray/level-11-x-ray.csv) | [+152,557 / 2.251 / 1.235](l11/level-11-mozilla.csv) |
| 12 | [+40,889 / 1.147 / 1.068](l12/level-12-enwik8.csv) | [+1,793,787 / 35.600 / 2.770](tracks/x-ray/level-12-x-ray.csv) | [+200,136 / 2.193 / 1.306](l12/level-12-mozilla.csv) |
| 13 | [+245,737 / 2.519 / 1.337](l13/level-13-enwik8.csv) | [+1,791,266 / 31.412 / 2.703](tracks/x-ray/level-13-x-ray.csv) | [+218,083 / 1.874 / 1.093](l13/level-13-mozilla.csv) |
| 14 | [+594,759 / 2.743 / 1.260](l14/level-14-enwik8.csv) | [+1,791,250 / 34.326 / 2.754](tracks/x-ray/level-14-x-ray.csv) | [+267,426 / 2.173 / 1.202](l14/level-14-mozilla.csv) |
| 15 | [+595,517 / 2.988 / 1.197](l15/level-15-enwik8.csv) | [+1,791,250 / 30.861 / 2.801](tracks/x-ray/level-15-x-ray.csv) | [+270,907 / 2.689 / 1.216](l15/level-15-mozilla.csv) |
| 16 | [+269,999 / 2.786 / 1.107](l16/level-16-enwik8.csv) | [+1,718,579 / 40.688 / 2.637](tracks/x-ray/level-16-x-ray.csv) | [+69,141 / 2.955 / 1.063](l16/level-16-mozilla.csv) |
| 17 | [+148,032 / 3.850 / 1.287](l17/level-17-enwik8.csv) | [+1,705,753 / 43.285 / 2.584](tracks/x-ray/level-17-x-ray.csv) | [+61,739 / 3.347 / 1.210](l17/level-17-mozilla.csv) |
| 18 | [+135,668 / 2.178 / 1.294](l18/level-18-enwik8.csv) | [+1,577,114 / 51.841 / 2.399](tracks/x-ray/level-18-x-ray.csv) | [+43,818 / 3.294 / 1.011](l18/level-18-mozilla.csv) |
| 19 | [+6,923 / 2.248 / 1.267](l19/level-19-enwik8.csv) | [+1,571,708 / 50.590 / 2.373](tracks/x-ray/level-19-x-ray.csv) | [+51,613 / 3.706 / 1.010](l19/level-19-mozilla.csv) |
| 20 | [+11,842 / 2.469 / 1.258](l20/level-20-enwik8.csv) | [+1,571,708 / 52.398 / 2.421](tracks/x-ray/level-20-x-ray.csv) | [+48,901 / 3.740 / 1.008](l20/level-20-mozilla.csv) |
| 21 | [+6,838 / 2.513 / 1.131](l21/level-21-enwik8.csv) | [+1,571,708 / 54.172 / 2.407](tracks/x-ray/level-21-x-ray.csv) | [+73,825 / 3.828 / 1.028](l21/level-21-mozilla.csv) |
| 22 | [+19,962 / 2.531 / 1.163](l22/level-22-enwik8.csv) | [+1,571,708 / 57.108 / 2.313](tracks/x-ray/level-22-x-ray.csv) | [+82,202 / 4.605 / 1.042](l22/level-22-mozilla.csv) |

| Closest measured comparison | File and level | Value |
|---|---|---:|
| Smallest byte margin | [enwik8, 6](l06/level-06-enwik8.csv) | +1,835 bytes |
| Lowest encode rate relative to zstd | [mozilla, 1](../../native-combined-1/level-01-mozilla.csv) | 1.0183× |
| Lowest decode rate relative to zstd | [enwik8, 9](l09/level-09-enwik8.csv) | 0.9686× |

Archive size never increased between successive presets. Counts of distinct sizes: 22 for enwik8, 5 for x-ray, 22 for Mozilla.
