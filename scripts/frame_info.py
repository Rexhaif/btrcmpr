#!/usr/bin/env python3
"""Read the framing and report the actual stored codec of each kind of block."""

import json
import mmap
from collections import Counter
from pathlib import Path
from struct import unpack_from

NAMES = {
    0: "raw",
    1: "LZ",
    2: "16-bit group predictor",
    4: "records",
    5: "mixed subblocks",
    6: "adjacent 16-bit predictor",
    7: "word dictionary",
    8: "instruction streams",
    9: "record columns",
    10: "entropy-coded adjacent 16-bit predictor",
    11: "sorted data",
    12: "instruction banks",
    13: "LZ with extended distances",
    16: "short-match LZ",
    17: "short-match LZ with extended distances",
    21: "adaptive instruction banks",
    33: "compact instruction banks",
    41: "paged LZ",
    42: "LZ with distance spans",
    43: "phrase grammar",
    48: "local phrase dictionary",
    50: "shared phrase dictionary and first token stream",
    51: "shared dictionary token stream",
    63: "learned 16-bit row predictor",
}


def inspect(path):
    """Charge all outer headers and payloads. The real decoder validates the payloads."""
    with (
        Path(path).open("rb") as fh,
        mmap.mmap(fh.fileno(), 0, access=mmap.ACCESS_READ) as data,
    ):
        if len(data) < 20 or data[:8] != b"BTRCMP01":
            raise ValueError("not a btrcmpr frame")
        raw, word = unpack_from("<QI", data, 8)
        limit = word & 0x7FFFFFFF
        if not 1 <= limit <= 32 << 20:
            raise ValueError("invalid job size")
        pos = 20

        def u32():
            nonlocal pos
            if pos + 4 > len(data):
                raise ValueError("truncated header")
            (n,) = unpack_from("<I", data, pos)
            pos += 4
            return n

        def var():
            nonlocal pos
            n = 0
            for shift in range(0, 64, 7):
                if pos >= len(data):
                    raise ValueError("truncated variable-length integer")
                b = data[pos]
                pos += 1
                if shift == 63 and b > 1:
                    break
                n |= (b & 127) << shift
                if b < 128:
                    return n
            raise ValueError("oversized variable-length integer")

        pieces = 0
        if word >> 31:
            count = u32()
            if not 0 < count <= len(data) - pos:
                raise ValueError("invalid layout job count")
            sizes = [var() for _ in range(count)]
            if sum(sizes) != raw or any(not 0 < n <= 32 << 20 for n in sizes):
                raise ValueError("layout job sizes disagree with original size")
            pieces = u32()
            if not 0 < pieces <= (len(data) - pos) // 2:
                raise ValueError("invalid layout piece count")
            for _ in range(pieces):
                var()
                var()
        else:
            count = (raw + limit - 1) // limit
            sizes = (min(limit, raw - i * limit) for i in range(count))
        if count > (len(data) - pos) // 5:
            raise ValueError("too few bytes for the declared blocks")
        header = pos + 4 * count
        counts, originals, payloads, strides = (
            Counter(),
            Counter(),
            Counter(),
            Counter(),
        )
        stored_dictionaries = 0
        for size in sizes:
            n = u32()
            if not 1 <= n <= limit + 1 or pos + n > len(data):
                raise ValueError("invalid block length")
            tag = data[pos]
            counts[tag] += 1
            originals[tag] += size
            payloads[tag] += n
            if tag == 63:
                if n < 4:
                    raise ValueError("truncated row predictor header")
                (stride,) = unpack_from("<H", data, pos + 2)
                strides[stride] += 1
            if tag == 50:
                if n < 6:
                    raise ValueError("truncated shared dictionary header")
                (length,) = unpack_from("<I", data, pos + 2)
                if length > n - 6:
                    raise ValueError("truncated shared dictionary")
                stored_dictionaries += length
            pos += n
        if pos != len(data) or header + sum(payloads.values()) != len(data):
            raise ValueError("block lengths do not cover the archive")
        return {
            "original_bytes": raw,
            "archive_bytes": len(data),
            "blocks": count,
            "layout_pieces": pieces,
            "framing_bytes": header,
            "embedded_dictionary_bytes": stored_dictionaries,
            "row_strides_in_samples": dict(sorted(strides.items())),
            "representations": [
                {
                    "tag": t,
                    "name": NAMES.get(t, "other"),
                    "blocks": counts[t],
                    "original_bytes": originals[t],
                    "payload_bytes": payloads[t],
                }
                for t in sorted(counts)
            ],
        }


if __name__ == "__main__":
    import argparse

    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("archive", type=Path)
    print(json.dumps(inspect(p.parse_args().archive), indent=2))
