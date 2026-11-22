#!/usr/bin/env python3
"""Build reproducible compiler-training inputs independently of the benchmark corpus."""

import argparse
import hashlib
import io
import json
import random
import struct
import tarfile
from pathlib import Path

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--out", type=Path, required=True)
a = p.parse_args()
out = a.out.resolve()
out.mkdir(parents=True, exist_ok=True)


def sha(b):
    return hashlib.sha256(b).hexdigest()


source = {}


def gather(group, paths, cap):
    data = bytearray()
    for path in sorted(paths):
        if len(data) >= cap:
            break
        if not path.is_file():
            continue
        raw = path.read_bytes()
        used = min(len(raw), cap - len(data))
        data += raw[:used]
        source[str(path)] = {
            "input": group,
            "size": len(raw),
            "used": used,
            "sha256": sha(raw),
        }
    if len(data) < 65536:
        raise ValueError("missing independent training data: " + group)
    (out / group).write_bytes(data)
    return bytes(data)


docs = gather("manuals", Path("/usr/share/doc/git/RelNotes").glob("*.txt"), 4 << 20)
py = gather("modules", Path("/usr/lib/python3.13").glob("*.py"), 4 << 20)
machine = gather(
    "executables",
    map(Path, ["/usr/bin/bash", "/usr/bin/git", "/usr/bin/python3.13"]),
    7 << 20,
)
rng = random.Random(0x5863)
signal = bytearray()
for i in range(400000):
    v = 3000 + ((i // 800) % 250) * 150 + ((i // 3) % 530) + rng.randrange(-10, 11)
    signal.extend(struct.pack("<H", v))
(out / "samples").write_bytes(signal)
with tarfile.open(out / "records.tar", "w", format=tarfile.USTAR_FORMAT) as t:
    for k in range(48):
        fields = bytearray()
        for j in range(1300 + k % 71):
            fields.extend(struct.pack("<III", j * 16 + k, j // 8, rng.randrange(12)))
        code = machine[(k * 7013) % 2000000 :][: 21023 + (k % 15) * 90]
        text = docs[(k * 10083) % (len(docs) - 33000) :][:32011]
        misc = py[k * 599 :][:8191]
        sections = [
            (b".text", code),
            (b".rdata", text),
            (b".pdata", fields),
            (b".data", misc),
        ]
        b = bytearray(24 + len(sections) * 64)
        struct.pack_into("<HH", b, 0, 0x183, len(sections))
        for j, (name, v) in enumerate(sections):
            at = 24 + j * 64
            b[at : at + len(name)] = name
            struct.pack_into("<QQ", b, at + 24, len(v), len(b))
            b.extend(v)
        e = tarfile.TarInfo(f"unit-{k:02}.o")
        e.size = len(b)
        e.mtime = 0
        t.addfile(e, io.BytesIO(b))
meta = {
    "kind": "compiler branch counts only; no model or dictionary read by the codec",
    "seed": 0x5863,
    "sources": source,
    "inputs": {
        n: {"sha256": sha((out / n).read_bytes()), "size": (out / n).stat().st_size}
        for n in ("manuals", "modules", "executables", "samples", "records.tar")
    },
}
(out / "training.json").write_text(json.dumps(meta, indent=2) + "\n")
