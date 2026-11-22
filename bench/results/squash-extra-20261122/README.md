# Complete comparison on the other 25 Squash files

The [Squash Benchmark](https://quixdb.github.io/squash-benchmark/) lists 28 files. These are all 25 outside the three original tuning inputs (enwik8, x-ray and Mozilla). **100/100 independently verified comparisons** for presets `-1`, `-3`, `-10`, `-22`. The published corpus used here totals 156,100,812 uncompressed bytes.

The frozen codec passed all three requirements in **4/100** comparisons with the decode allowance (**3/100** with strictly faster decode). 4 of the 25 files passed at any tested level: mr, ooffice, osdb, webster. [Final verification](verification.json).

[At-a-glance outcomes for all 25 files](outcomes.md) · [All measured ratios, absolute MB/s, sizes, and speedups](comparisons.csv) · [Aggregate data](by-level.csv) · [Input provenance and hashes](datasets.json) · [Frozen benchmark settings](method.json) · [Raw paired trials](raw/) · [Standalone archive and restoration checks](checks/).

Each count is the number of files where the frozen Rust codec beats matched zstd. Size and encode require strict wins; decode may be up to 5% slower in the “all, 5%” column. “All strict” also requires strictly faster decode.

| Preset | Files | Smaller | Faster encode | Faster decode | Decode ≥95% | All, 5% | All strict |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 25 | 5 | 5 | 9 | 13 | 0 | 0 |
| 3 | 25 | 11 | 7 | 11 | 13 | 4 | 3 |
| 10 | 25 | 4 | 2 | 10 | 14 | 0 | 0 |
| 22 | 25 | 2 | 12 | 16 | 16 | 0 | 0 |

The next table treats each file as a separate compression operation: compression ratio is total raw bytes divided by total archive bytes, and throughput divides total raw bytes by the sum of file median times. Larger files carry more weight. The files were never concatenated. Each pair is **Rust / zstd**.

| Preset | Ratio (×) | Encode MB/s | Decode MB/s |
|---:|---:|---:|---:|
| 1 | 3.1476 / 3.2745 | 706.62 / 719.43 | 1685.54 / 1674.15 |
| 3 | 3.6496 / 3.6255 | 126.75 / 112.16 | 1450.65 / 1282.34 |
| 10 | 3.9424 / 4.0926 | 15.36 / 19.24 | 1185.45 / 1101.31 |
| 22 | 4.2881 / 4.4923 | 3.53 / 3.61 | 1063.18 / 1035.86 |

Passing comparisons by source corpus. Each entry counts files passing all three requirements with the 5% decode allowance. [Counts by metric and source](by-corpus.csv).

| Preset | Canterbury | Silesia | Snappy |
|---:|---:|---:|---:|
| 1 | 0 / 11 | 0 / 10 | 0 / 4 |
| 3 | 0 / 11 | 4 / 10 | 0 / 4 |
| 10 | 0 / 11 | 0 / 10 | 0 / 4 |
| 22 | 0 / 11 | 0 / 10 | 0 / 4 |

## Per-file results

Ratios are original bytes divided by archived bytes (higher is better). Speeds use decimal MB/s of original data. Each row is **Rust / zstd**; the last column names failed requirements. A compressed file may be larger than its input. On the smallest inputs, worker startup and scheduling can dominate.

### Preset 1

| File | Ratio (×) | Encode MB/s | Decode MB/s | 5% test |
|---|---:|---:|---:|---|
| [alice29.txt](raw/01-alice29.txt.csv) | 2.2763 / 2.3775 | 335.31 / 419.45 | 1002.53 / 1003.05 | size, encode |
| [asyoulik.txt](raw/01-asyoulik.txt.csv) | 2.1320 / 2.2140 | 314.53 / 406.37 | 903.19 / 944.91 | size, encode |
| [cp.html](raw/01-cp.html.csv) | 2.1344 / 2.4306 | 123.43 / 167.98 | 294.53 / 283.25 | size, encode |
| [dickens](raw/01-dickens.csv) | 2.3777 / 2.3891 | 434.57 / 403.44 | 1330.39 / 1405.59 | size, decode |
| [fields.c](raw/01-fields.c.csv) | 2.0787 / 2.5900 | 60.70 / 96.37 | 111.22 / 127.40 | size, encode, decode |
| [fireworks.jpeg](raw/01-fireworks.jpeg.csv) | 0.9998 / 0.9994 | 273.83 / 593.80 | 1084.74 / 1380.35 | encode, decode |
| [geo.protodata](raw/01-geo.protodata.csv) | 4.9714 / 5.6379 | 212.52 / 514.44 | 553.26 / 1086.08 | size, encode, decode |
| [grammar.lsp](raw/01-grammar.lsp.csv) | 1.6299 / 2.2109 | 18.92 / 36.08 | 40.46 / 47.47 | size, encode, decode |
| [kennedy.xls](raw/01-kennedy.xls.csv) | 6.7107 / 9.1112 | 758.54 / 793.38 | 2094.03 / 2075.89 | size, encode |
| [lcet10.txt](raw/01-lcet10.txt.csv) | 2.5622 / 2.5964 | 453.39 / 566.58 | 1460.67 / 1660.75 | size, encode, decode |
| [mr](raw/01-mr.csv) | 2.5509 / 2.6144 | 572.97 / 609.55 | 1446.92 / 1439.75 | size, encode |
| [nci](raw/01-nci.csv) | 8.9904 / 11.7685 | 1316.53 / 1564.94 | 2122.93 / 2031.85 | size, encode |
| [ooffice](raw/01-ooffice.csv) | 1.7071 / 1.7171 | 509.19 / 606.73 | 1268.79 / 1281.28 | size, encode |
| [osdb](raw/01-osdb.csv) | 2.7093 / 2.6946 | 732.22 / 675.67 | 1581.43 / 1670.97 | decode |
| [paper-100k.pdf](raw/01-paper-100k.pdf.csv) | 1.2321 / 1.2278 | 337.29 / 436.84 | 1250.18 / 1218.24 | encode |
| [plrabn12.txt](raw/01-plrabn12.txt.csv) | 2.1914 / 2.1808 | 438.14 / 519.07 | 1467.12 / 1495.90 | encode |
| [ptt5](raw/01-ptt5.csv) | 8.3245 / 9.9725 | 629.78 / 990.37 | 1995.11 / 2955.07 | size, encode, decode |
| [reymont](raw/01-reymont.csv) | 2.9609 / 3.0775 | 490.27 / 470.14 | 1537.78 / 1478.29 | size |
| [samba](raw/01-samba.csv) | 3.6343 / 3.9369 | 837.28 / 865.36 | 1963.15 / 1893.55 | size, encode |
| [sao](raw/01-sao.csv) | 1.2015 / 1.1594 | 452.27 / 531.41 | 1115.36 / 1184.37 | encode, decode |
| [sum](raw/01-sum.csv) | 2.1639 / 2.4428 | 139.50 / 181.33 | 301.18 / 422.26 | size, encode, decode |
| [urls.10K](raw/01-urls.10K.csv) | 3.0721 / 3.2020 | 618.65 / 723.42 | 1756.37 / 2090.94 | size, encode, decode |
| [xargs.1](raw/01-xargs.1.csv) | 1.4025 / 1.8064 | 26.52 / 45.13 | 49.62 / 52.96 | size, encode, decode |
| [webster](raw/01-webster.csv) | 2.9566 / 3.0334 | 698.44 / 651.06 | 1759.10 / 1696.71 | size |
| [xml](raw/01-xml.csv) | 6.1325 / 7.6347 | 976.83 / 893.88 | 1990.68 / 1988.25 | size |

### Preset 3

| File | Ratio (×) | Encode MB/s | Decode MB/s | 5% test |
|---|---:|---:|---:|---|
| [alice29.txt](raw/03-alice29.txt.csv) | 2.4835 / 2.5088 | 61.91 / 143.62 | 878.46 / 931.19 | size, encode, decode |
| [asyoulik.txt](raw/03-asyoulik.txt.csv) | 2.2824 / 2.3190 | 57.46 / 122.01 | 865.52 / 790.44 | size, encode |
| [cp.html](raw/03-cp.html.csv) | 2.2312 / 2.4724 | 26.26 / 117.26 | 225.22 / 279.95 | size, encode, decode |
| [dickens](raw/03-dickens.csv) | 2.7519 / 2.7645 | 82.32 / 80.12 | 1192.32 / 977.65 | size |
| [fields.c](raw/03-fields.c.csv) | 2.1726 / 2.6809 | 16.28 / 81.11 | 115.46 / 137.16 | size, encode, decode |
| [fireworks.jpeg](raw/03-fireworks.jpeg.csv) | 0.9999 / 0.9994 | 43.15 / 244.71 | 992.42 / 1408.39 | encode, decode |
| [geo.protodata](raw/03-geo.protodata.csv) | 5.5052 / 5.8400 | 87.25 / 178.54 | 595.50 / 1138.99 | size, encode, decode |
| [grammar.lsp](raw/03-grammar.lsp.csv) | 1.7355 / 2.2814 | 6.78 / 33.67 | 47.59 / 42.40 | size, encode |
| [kennedy.xls](raw/03-kennedy.xls.csv) | 7.2425 / 9.2915 | 118.17 / 259.88 | 1869.76 / 2055.53 | size, encode, decode |
| [lcet10.txt](raw/03-lcet10.txt.csv) | 2.8094 / 2.7905 | 88.32 / 199.81 | 1619.11 / 1538.75 | encode |
| [mr](raw/03-mr.csv) | 2.9000 / 2.8083 | 99.79 / 81.72 | 1206.65 / 1129.44 | Pass |
| [nci](raw/03-nci.csv) | 11.1883 / 11.8201 | 372.95 / 441.94 | 1958.54 / 1810.25 | size, encode |
| [ooffice](raw/03-ooffice.csv) | 1.9725 / 1.9587 | 82.87 / 60.13 | 1180.08 / 1198.64 | Pass |
| [osdb](raw/03-osdb.csv) | 2.9280 / 2.8689 | 112.77 / 75.59 | 1355.17 / 1294.20 | Pass |
| [paper-100k.pdf](raw/03-paper-100k.pdf.csv) | 1.2398 / 1.2312 | 47.18 / 124.22 | 804.60 / 1074.17 | encode, decode |
| [plrabn12.txt](raw/03-plrabn12.txt.csv) | 2.3701 / 2.3615 | 75.62 / 102.36 | 1649.51 / 1413.82 | encode |
| [ptt5](raw/03-ptt5.csv) | 8.8981 / 9.7320 | 184.54 / 389.10 | 2206.38 / 2483.60 | size, encode, decode |
| [reymont](raw/03-reymont.csv) | 3.4866 / 3.3916 | 104.78 / 125.31 | 1295.24 / 1059.37 | encode |
| [samba](raw/03-samba.csv) | 4.1884 / 4.3431 | 164.13 / 159.20 | 1750.24 / 1561.48 | size |
| [sao](raw/03-sao.csv) | 1.3572 / 1.3095 | 66.03 / 45.18 | 956.67 / 1031.24 | decode |
| [sum](raw/03-sum.csv) | 2.2651 / 2.5000 | 34.99 / 123.21 | 278.73 / 403.37 | size, encode, decode |
| [urls.10K](raw/03-urls.10K.csv) | 3.3103 / 3.2998 | 103.86 / 196.24 | 1286.65 / 1879.48 | encode, decode |
| [xargs.1](raw/03-xargs.1.csv) | 1.3974 / 1.8283 | 7.57 / 44.23 | 46.14 / 51.09 | size, encode, decode |
| [webster](raw/03-webster.csv) | 3.4912 / 3.4235 | 107.17 / 93.13 | 1373.13 / 1074.55 | Pass |
| [xml](raw/03-xml.csv) | 8.3348 / 8.3671 | 292.30 / 335.19 | 2049.51 / 2082.40 | size, encode |

### Preset 10

| File | Ratio (×) | Encode MB/s | Decode MB/s | 5% test |
|---|---:|---:|---:|---|
| [alice29.txt](raw/10-alice29.txt.csv) | 2.4637 / 2.6573 | 2.61 / 47.08 | 849.48 / 856.23 | size, encode |
| [asyoulik.txt](raw/10-asyoulik.txt.csv) | 2.2519 / 2.4286 | 2.17 / 71.16 | 709.87 / 618.90 | size, encode |
| [cp.html](raw/10-cp.html.csv) | 2.2638 / 2.5698 | 10.66 / 31.00 | 109.48 / 175.99 | size, encode, decode |
| [dickens](raw/10-dickens.csv) | 3.0296 / 3.0878 | 13.52 / 11.58 | 874.94 / 802.02 | size |
| [fields.c](raw/10-fields.c.csv) | 2.2097 / 2.8199 | 8.02 / 41.84 | 99.42 / 120.79 | size, encode, decode |
| [fireworks.jpeg](raw/10-fireworks.jpeg.csv) | 0.9999 / 0.9994 | 18.94 / 199.17 | 866.05 / 1268.86 | encode, decode |
| [geo.protodata](raw/10-geo.protodata.csv) | 5.6142 / 6.1390 | 41.29 / 137.38 | 604.29 / 1224.29 | size, encode, decode |
| [grammar.lsp](raw/10-grammar.lsp.csv) | 1.7677 / 2.3551 | 5.44 / 25.11 | 43.63 / 44.33 | size, encode |
| [kennedy.xls](raw/10-kennedy.xls.csv) | 8.3555 / 9.2174 | 37.20 / 42.76 | 2016.69 / 1793.07 | size, encode |
| [lcet10.txt](raw/10-lcet10.txt.csv) | 2.8438 / 2.9805 | 5.56 / 44.36 | 1347.52 / 1497.33 | size, encode, decode |
| [mr](raw/10-mr.csv) | 3.0299 / 2.9890 | 12.31 / 17.61 | 1106.59 / 904.85 | encode |
| [nci](raw/10-nci.csv) | 12.5247 / 15.3223 | 56.42 / 59.08 | 1953.95 / 1670.74 | size, encode |
| [ooffice](raw/10-ooffice.csv) | 2.0781 / 2.1029 | 6.42 / 15.63 | 985.03 / 1023.29 | size, encode |
| [osdb](raw/10-osdb.csv) | 2.9846 / 3.0401 | 11.45 / 13.86 | 1237.17 / 1173.35 | size, encode |
| [paper-100k.pdf](raw/10-paper-100k.pdf.csv) | 1.2415 / 1.2413 | 24.63 / 96.38 | 832.87 / 980.99 | encode, decode |
| [plrabn12.txt](raw/10-plrabn12.txt.csv) | 2.3941 / 2.4954 | 5.88 / 43.08 | 1411.76 / 1021.31 | size, encode |
| [ptt5](raw/10-ptt5.csv) | 9.1833 / 10.2312 | 79.65 / 114.71 | 2116.43 / 2549.23 | size, encode, decode |
| [reymont](raw/10-reymont.csv) | 3.8660 / 3.9693 | 14.73 / 19.84 | 1160.14 / 1186.32 | size, encode |
| [samba](raw/10-samba.csv) | 4.4197 / 4.8935 | 18.61 / 27.62 | 1615.20 / 1517.32 | size, encode |
| [sao](raw/10-sao.csv) | 1.4116 / 1.3781 | 4.16 / 11.95 | 858.10 / 849.03 | encode |
| [sum](raw/10-sum.csv) | 2.3082 / 2.5971 | 18.71 / 48.96 | 259.27 / 395.23 | size, encode, decode |
| [urls.10K](raw/10-urls.10K.csv) | 3.3488 / 3.5505 | 7.99 / 25.47 | 1388.19 / 1805.50 | size, encode, decode |
| [xargs.1](raw/10-xargs.1.csv) | 1.4053 / 1.8572 | 3.68 / 32.29 | 40.18 / 50.40 | size, encode, decode |
| [webster](raw/10-webster.csv) | 3.9674 / 4.0839 | 18.37 / 13.52 | 910.97 / 842.26 | size |
| [xml](raw/10-xml.csv) | 8.7590 / 10.4348 | 39.26 / 51.46 | 1787.07 / 2027.94 | size, encode, decode |

### Preset 22

| File | Ratio (×) | Encode MB/s | Decode MB/s | 5% test |
|---|---:|---:|---:|---|
| [alice29.txt](raw/22-alice29.txt.csv) | 2.5725 / 2.7424 | 6.92 / 8.01 | 928.92 / 811.62 | size, encode |
| [asyoulik.txt](raw/22-asyoulik.txt.csv) | 2.3426 / 2.4966 | 6.34 / 10.55 | 800.57 / 687.65 | size, encode |
| [cp.html](raw/22-cp.html.csv) | 2.2715 / 2.6093 | 7.78 / 11.80 | 225.03 / 248.85 | size, encode, decode |
| [dickens](raw/22-dickens.csv) | 3.6760 / 3.3723 | 4.77 / 3.29 | 711.40 / 810.77 | decode |
| [fields.c](raw/22-fields.c.csv) | 2.2154 / 2.9006 | 4.25 / 9.81 | 89.65 / 105.20 | size, encode, decode |
| [fireworks.jpeg](raw/22-fireworks.jpeg.csv) | 0.9999 / 1.0011 | 15.39 / 26.39 | 951.11 / 1041.85 | size, encode, decode |
| [geo.protodata](raw/22-geo.protodata.csv) | 5.6033 / 6.4517 | 26.44 / 2.44 | 624.40 / 979.76 | size, decode |
| [grammar.lsp](raw/22-grammar.lsp.csv) | 1.7585 / 2.3929 | 2.45 / 10.25 | 46.50 / 44.91 | size, encode |
| [kennedy.xls](raw/22-kennedy.xls.csv) | 8.4114 / 12.5034 | 4.50 / 1.83 | 1704.85 / 1516.36 | size |
| [lcet10.txt](raw/22-lcet10.txt.csv) | 2.9514 / 3.1142 | 6.50 / 6.34 | 1517.10 / 1187.42 | size |
| [mr](raw/22-mr.csv) | 3.0346 / 3.1317 | 2.08 / 4.82 | 1038.04 / 859.43 | size, encode |
| [nci](raw/22-nci.csv) | 14.1671 / 19.2802 | 5.48 / 3.22 | 2016.97 / 1582.45 | size |
| [ooffice](raw/22-ooffice.csv) | 2.0888 / 2.3032 | 2.18 / 6.61 | 923.55 / 902.88 | size, encode |
| [osdb](raw/22-osdb.csv) | 3.0760 / 3.1472 | 2.35 / 5.31 | 1158.29 / 981.86 | size, encode |
| [paper-100k.pdf](raw/22-paper-100k.pdf.csv) | 1.2419 / 1.2611 | 16.26 / 3.95 | 823.24 / 946.38 | size, decode |
| [plrabn12.txt](raw/22-plrabn12.txt.csv) | 2.4813 / 2.6344 | 6.14 / 5.41 | 1631.74 / 1292.30 | size |
| [ptt5](raw/22-ptt5.csv) | 9.4531 / 11.7594 | 35.89 / 2.16 | 2334.34 / 2556.53 | size, decode |
| [reymont](raw/22-reymont.csv) | 4.0970 / 4.5717 | 1.18 / 4.30 | 1115.21 / 1079.51 | size, encode |
| [samba](raw/22-samba.csv) | 4.6594 / 5.3094 | 3.68 / 3.94 | 1742.07 / 1540.29 | size, encode |
| [sao](raw/22-sao.csv) | 1.4158 / 1.4456 | 1.44 / 5.70 | 791.71 / 791.20 | size, encode |
| [sum](raw/22-sum.csv) | 2.3142 / 2.9213 | 9.83 / 3.24 | 264.02 / 348.90 | size, decode |
| [urls.10K](raw/22-urls.10K.csv) | 3.5200 / 3.6632 | 5.94 / 5.60 | 1987.02 / 1887.15 | size |
| [xargs.1](raw/22-xargs.1.csv) | 1.4137 / 1.8704 | 1.50 / 7.13 | 30.11 / 25.39 | size, encode |
| [webster](raw/22-webster.csv) | 4.7229 / 4.6399 | 7.41 / 3.10 | 720.43 / 784.90 | decode |
| [xml](raw/22-xml.csv) | 10.5770 / 11.6348 | 3.95 / 2.77 | 1966.15 / 1897.06 | size |

## Method

On the four-core Cortex-A76 Raspberry Pi 5, both encoders and both decoders received a worker count of four. The pinned comparator and standalone Rust CLI are the same files as in the [original release](../presets/native-combined-1/release.md). The reference was libzstd 1.5.7 with a separate full quarter-file frame per worker so decoding can use all four workers; all framing, including the outer directory, counts toward size. This measures that baseline, not the stock single-stream CLI. Each file had one warmup pair and five alternating measured pairs per level, with seven decodes per timing. The median encode and decode times were taken separately for each codec. Each timed pair verified exact decoded bytes, and the standalone CLI independently matched the archive size and restored the original SHA-256. The timings exclude disk I/O. Inputs were decoded from the Squash project’s pinned XZ files, verified against Git blob IDs and the website’s original-byte sizes; no input slicing or content changes were used.

To repeat one measurement with the existing binary, from the repository root:

```sh
BTR_LEVEL=3 ZSTD_LEVEL=3 ZSTD_WIDE=1 WARMUP=1 DEC_ITERS=7 TRIALS_CSV=/tmp/squash-pairs.csv \
  bench/local/preset-binaries/f8d320e7b5b2449a2586eccc6af365eafb306215e6c6b403a91de57e03078464 4 5 both bench/data/squash-extra/dickens
```

The zstd size includes a 36-byte outer directory (one 20-byte header and four lengths) for each input. Simply concatenating the same four zstd frames would omit it. Removing those 36 bytes changes the size verdict on paper-100k.pdf at -10 (Rust saved 14 B with the directory). It would also change 0 combined verdict(s). This adjustment does not estimate single-frame zstd.
