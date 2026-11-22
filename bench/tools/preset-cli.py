#!/usr/bin/env python3
"""Check CLI archive sizes and round trips against benchmark results."""

import argparse
import csv
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("matrix", type=Path, nargs="+")
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--binary", type=Path, help="path to the standalone encoder")
parser.add_argument("--sha256", help="expected SHA-256 of --binary")
args = parser.parse_args()
if bool(args.binary) != bool(args.sha256):
    parser.error("--binary and --sha256 must be supplied together")
root = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(root / "scripts"))
from corpus import INPUTS

mix = []
inspect_frame = None
if args.binary and (root / "scripts/frame_info.py").is_file():
    from frame_info import inspect as inspect_frame
if args.binary:
    exe = args.binary.resolve()
    with exe.open("rb") as fh:
        if hashlib.file_digest(fh, "sha256").hexdigest() != args.sha256:
            raise SystemExit("frozen standalone executable has changed")
else:
    exe = (root / os.environ.get("CARGO_TARGET_DIR", "target")) / "release/btrcmpr"
    subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "--offline",
            "--locked",
            "--no-default-features",
            "--bin",
            "btrcmpr",
        ],
        cwd=root,
        check=True,
    )
if "libzstd" in subprocess.check_output(["ldd", exe], text=True):
    raise SystemExit("production binary unexpectedly links libzstd")


def digest(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def same_file(a, b):
    with a.open("rb") as x, b.open("rb") as y:
        while True:
            first = x.read(1 << 20)
            if first != y.read(1 << 20):
                return False
            if not first:
                return True


result = io.StringIO()
writer = csv.writer(result, lineterminator="\n")
exe_sha = digest(exe)
writer.writerow(
    (
        "level",
        "input",
        "cli_bytes",
        "source_sha256",
        "archive_sha256",
        "restored_identical",
        "cli_sha256",
    )
)
with tempfile.TemporaryDirectory(dir=root / "target") as work:
    folder = Path(work)
    archive, restored = folder / "archive.btr", folder / "output.bin"
    for matrix in args.matrix:
        with matrix.open() as handle:
            for row in csv.DictReader(handle):
                level, name, threads = row["level"], row["file"], row["threads"]
                source = root / INPUTS[name].path
                subprocess.run(
                    [exe, "c", "-" + level, source, archive, threads], check=True
                )
                subprocess.run([exe, "d", archive, restored, threads], check=True)
                size = archive.stat().st_size
                if size != int(row["btr_bytes"]) or not same_file(source, restored):
                    raise SystemExit(
                        f"CLI size or restored data differ at {name} level {level}"
                    )
                archive_sha = digest(archive)
                writer.writerow(
                    (level, name, size, digest(source), archive_sha, 1, exe_sha)
                )
                if inspect_frame:
                    info = inspect_frame(archive)
                    if (
                        info["original_bytes"] != source.stat().st_size
                        or info["archive_bytes"] != size
                    ):
                        raise ValueError(
                            "block accounting and round-trip lengths disagree"
                        )
                    mix.append(
                        dict(
                            level=int(level),
                            file=name,
                            archive_sha256=archive_sha,
                            cli_sha256=exe_sha,
                            **info,
                        )
                    )
args.out.parent.mkdir(parents=True, exist_ok=True)
with tempfile.NamedTemporaryFile(
    mode="w", dir=args.out.parent, delete=False
) as pending:
    pending.write(result.getvalue())
os.replace(pending.name, args.out)
if mix:
    with tempfile.NamedTemporaryFile(
        mode="w", dir=args.out.parent, delete=False
    ) as pending:
        json.dump(mix, pending, indent=2)
        pending.write("\n")
    os.replace(pending.name, args.out.with_suffix(".blocks.json"))
