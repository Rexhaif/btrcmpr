"""Apply size and throughput thresholds to paired medians."""

import csv
import os
import tempfile
from decimal import Decimal, InvalidOperation

DEFAULT_DECODE_LOSS_PCT = Decimal(5)


def allowance(value):
    try:
        p = Decimal(value)
    except InvalidOperation as exc:
        raise ValueError("decode loss must be a finite percentage") from exc
    if not p.is_finite() or not 0 <= p < 100:
        raise ValueError("decode loss must be in [0, 100) percent")
    return p


def columns(p):
    return sorted({Decimal(0), Decimal(2), Decimal(5), Decimal(10), p})


def qualifies(costs, p):
    (size, enc, dec), (refsize, refenc, refdec) = costs
    # At 95% of reference throughput, the time limit is refdec / 0.95.
    dec_ok = dec < refdec if p == 0 else 100 * refdec >= (100 - p) * dec
    return size < refsize and enc < refenc and dec_ok


def label(p):
    return format(p.normalize(), "f")


def write_csv(path, header, rows):
    with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as tmp:
        w = csv.writer(tmp, lineterminator="\n")
        w.writerow(header)
        w.writerows(rows)
    os.replace(tmp.name, path)
