#!/usr/bin/env python3
"""Give absolute median rates and ratios for a verified preset matrix."""

import argparse
import csv
from pathlib import Path
from statistics import median

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("matrix", type=Path)
p.add_argument("--out", type=Path, required=True)
a = p.parse_args()
result = []
for row in csv.DictReader(a.matrix.open()):
    name = row["file"]
    lv = int(row["level"])
    n = int(row["rounds"])
    trials = list(
        csv.DictReader((a.matrix.parent / f"level-{lv:02}-{name}.csv").open())
    )
    expected = {
        c: int(row[k]) for c, k in [("btrcmpr", "btr_bytes"), ("zstd", "zstd_bytes")]
    }
    pair = []
    for codec, stored in expected.items():
        seq = [r for r in trials if r["codec"] == codec]
        if len(seq) != n or {int(r["round"]) for r in seq} != set(range(1, n + 1)):
            raise ValueError("incomplete trials")
        raw = {int(r["raw_bytes"]) for r in seq}
        if len(raw) != 1 or any(int(r["compressed_bytes"]) != stored for r in seq):
            raise ValueError("inconsistent lengths")
        size = raw.pop()
        mb = size / 1e6
        pair.append(
            [
                stored,
                size / stored,
                *(
                    mb / median(float(r[key]) for r in seq)
                    for key in ("compress_seconds", "decompress_seconds")
                ),
            ]
        )
    ours, base = pair
    if (
        abs(ours[2] / base[2] - float(row["encode_speedup"])) > 5.1e-5
        or abs(ours[3] / base[3] - float(row["decode_speedup"])) > 5.1e-5
    ):
        raise ValueError("absolute rates disagree with paired summary")
    result.append(
        [
            lv,
            name,
            n,
            ours[0],
            base[0],
            *(f"{v:.4f}" for v in (ours[1], base[1])),
            *(f"{v:.2f}" for v in (ours[2], base[2], ours[3], base[3])),
            row["all_pass"],
        ]
    )
a.out.parent.mkdir(parents=True, exist_ok=True)
with a.out.open("w") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(
        (
            "level",
            "file",
            "rounds",
            "btr_bytes",
            "zstd_bytes",
            "btr_ratio",
            "zstd_ratio",
            "btr_encode_MBps",
            "zstd_encode_MBps",
            "btr_decode_MBps",
            "zstd_decode_MBps",
            "all_pass",
        )
    )
    w.writerows(result)
