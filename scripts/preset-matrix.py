#!/usr/bin/env python3
"""Benchmark btrcmpr and zstd at matching levels."""

import argparse
import csv
import hashlib
import json
import math
import os
import re
import shutil
import statistics
import subprocess
import sys
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "bench/tools"))
from corpus import CORPORA, INPUTS
from decode_gate import DEFAULT_DECODE_LOSS_PCT, allowance, label, qualifies, write_csv

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--levels")
p.add_argument(
    "--corpus",
    choices=CORPORA,
    help="original three files (default) or the other 25 Squash files",
)
p.add_argument("--files", nargs="+", choices=INPUTS)
p.add_argument("--threads", type=int)
p.add_argument("--rounds", type=int)
p.add_argument("--decodes", type=int)
p.add_argument(
    "--selected",
    action="store_true",
    help="use the profiled binaries, check round trips and report MB/s",
)
p.add_argument("--out", type=Path, required=True)
p.add_argument("--summarize", action="store_true")
p.add_argument(
    "--allow-decode-loss-pct",
    type=allowance,
    default=DEFAULT_DECODE_LOSS_PCT,
    help="maximum decode throughput loss per file (default: 5%%); matrix.csv uses 0%%",
)
p.add_argument(
    "--from-manifest",
    type=Path,
    help="use the comparator recorded in MANIFEST",
)
p.add_argument(
    "--resume",
    action="store_true",
    help="resume with the recorded binaries, inputs and settings",
)
p.add_argument(
    "--screen",
    action="store_true",
    default=None,
    help="skip warmup (requires --rounds 1)",
)
args = p.parse_args()
os.chdir(ROOT)
out = args.out.resolve()
if sum(map(bool, (args.resume, args.summarize, args.from_manifest))) > 1 or (
    args.from_manifest and args.selected
):
    p.error(
        "choose one of --from-manifest, --resume or --summarize; --from-manifest conflicts with --selected"
    )
from benchmark_lock import run_trial, take_directory

out.mkdir(parents=True, exist_ok=True)
# Readers that regenerate reports also need the directory lock.
series_lock = take_directory(out, "output directory is in use")
saved = {}
if args.resume or args.summarize:
    manifest = out / "manifest.json"
    if not manifest.is_file():
        p.error("no matrix manifest in the output directory")
    saved = json.loads(manifest.read_text())
if (
    args.selected
    and (args.resume or args.summarize)
    and not saved.get("selected_build")
):
    p.error("the existing matrix was not started with --selected")
origin = json.loads(args.from_manifest.read_text()) if args.from_manifest else {}
selected = args.selected or bool(
    saved.get("selected_build") or origin.get("selected_build")
)
corpus_name = args.corpus or saved.get("corpus") or origin.get("corpus") or "original"
corpus = CORPORA[corpus_name]
if args.files and any(name not in corpus for name in args.files):
    p.error("requested input is not part of --corpus " + corpus_name)
if saved and corpus_name != saved.get("corpus", "original"):
    p.error("cannot change corpus for an existing matrix")
defaults = {
    "levels": "1,3,10,22" if corpus_name == "extra" else "1:22",
    "files": list(corpus),
    "threads": 4,
    "rounds": 5,
    "decodes": 7 if selected else 4,
    "screen": False,
}
if saved:
    defaults.update({k: saved[k] for k in ("files", "threads", "rounds", "decodes")})
    defaults.update(
        levels=",".join(map(str, saved["levels"])), screen=not saved["warmup_rounds"]
    )
for option, value in defaults.items():
    if getattr(args, option) is None:
        setattr(args, option, value)
levels = []
for part in args.levels.split(","):
    if not re.fullmatch(r"\d+(?::\d+)?", part):
        p.error("invalid level or range: " + part)
    bounds = list(map(int, part.split(":")))
    if bounds[0] > bounds[-1]:
        p.error("reversed level range: " + part)
    levels.extend(range(bounds[0], bounds[-1] + 1))
if (
    len(set(levels)) != len(levels)
    or not levels
    or min(levels) < 1
    or max(levels) > 22
    or min(args.threads, args.rounds, args.decodes) < 1
):
    p.error("invalid levels or benchmark parameters")
if len(args.files) != len(set(args.files)):
    p.error("duplicate input")
if (
    args.summarize
    and selected
    and (
        levels != saved["levels"]
        or args.files != saved["files"]
        or args.rounds != saved["rounds"]
        or args.threads != saved["threads"]
        or args.decodes != saved["decodes"]
        or int(not args.screen) != saved["warmup_rounds"]
    )
):
    p.error("summarize the selected series using the settings recorded in its manifest")
if args.screen and args.rounds != 1:
    p.error("screening uses exactly one measured round")


def file_digest(path):
    with Path(path).open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def runtime(binary):
    found = re.search(
        r"libzstd\.so\.1 => (\S+)", subprocess.check_output(["ldd", binary], text=True)
    )
    if found is None:
        sys.exit("benchmark does not link the zstd reference library")
    return {
        "zstd_library_sha256": file_digest(found[1]),
        "input_sha256": {
            name: file_digest(ROOT / corpus[name].path) for name in args.files
        },
    }


cli = None
if args.resume:
    meta = saved
    digest = meta["benchmark_sha256"]
    if not re.fullmatch(r"[a-f0-9]{64}", digest):
        sys.exit("invalid benchmark digest")
    binary = ROOT / "bench/local/preset-binaries" / digest
    if file_digest(binary) != digest:
        sys.exit("frozen benchmark executable changed")
    check = {
        "levels": levels,
        "files": args.files,
        "threads": args.threads,
        "rounds": args.rounds,
        "warmup_rounds": int(not args.screen),
        "decodes": args.decodes,
        "zstd_frame_policy": "one independent frame per worker",
        **runtime(binary),
    }
    changed = [k for k, v in check.items() if meta.get(k) != v]
    if changed:
        sys.exit(
            "cannot resume with mismatched or unrecorded settings: "
            + ", ".join(changed)
        )
elif not args.summarize:
    if (out / "manifest.json").exists() or any(out.glob("level-*.csv")):
        sys.exit(
            "output directory contains a benchmark; use --resume or a new directory"
        )

    def source_hashes():
        return {
            str(p.relative_to(ROOT)): file_digest(p)
            for p in sorted((ROOT / "src").rglob("*.rs"))
        }

    extra = {}
    if args.from_manifest:
        binary_hash = origin["benchmark_sha256"]
        if not re.fullmatch(r"[a-f0-9]{64}", binary_hash):
            sys.exit("invalid source benchmark digest")
        binary = ROOT / "bench/local/preset-binaries" / binary_hash
        if file_digest(binary) != binary_hash:
            sys.exit("frozen comparator differs from the original")
        runtime_id = runtime(binary)
        if runtime_id["zstd_library_sha256"] != origin.get(
            "zstd_library_sha256"
        ) or any(
            v != origin.get("input_sha256", {}).get(n)
            for n, v in runtime_id["input_sha256"].items()
        ):
            sys.exit("original corpus or zstd library identity does not match")
        hashes = origin["source_sha256"]
        if "selected_build" in origin:
            extra["selected_build"] = origin["selected_build"]
    elif args.selected:
        from selected_build import freeze

        hashes = source_hashes()
        binary, stamp = freeze(ROOT, hashes)
        if hashes != source_hashes():
            sys.exit("sources changed while preparing the selected build")
        binary_hash = stamp["comparator_sha256"]
        runtime_id = runtime(binary)
        extra["selected_build"] = stamp
    else:
        hashes = source_hashes()
        subprocess.run(
            [
                "cargo",
                "build",
                "--quiet",
                "--release",
                "--locked",
                "--features",
                "baseline",
                "--bin",
                "compare",
            ],
            check=True,
        )
        if hashes != source_hashes():
            sys.exit("sources changed during the build")
        original = (
            ROOT / os.environ.get("CARGO_TARGET_DIR", "target")
        ) / "release/compare"
        binary_hash = file_digest(original)
        binary = ROOT / "bench/local/preset-binaries" / binary_hash
        binary.parent.mkdir(parents=True, exist_ok=True)
        if not binary.exists():
            shutil.copy2(original, binary)
        if file_digest(binary) != binary_hash:
            sys.exit("benchmark executable does not match its digest")
        runtime_id = runtime(binary)
    meta = {
        "corpus": corpus_name,
        "benchmark_sha256": binary_hash,
        "levels": levels,
        "files": args.files,
        "threads": args.threads,
        "rounds": args.rounds,
        "warmup_rounds": int(not args.screen),
        "decodes": args.decodes,
        "zstd_frame_policy": "one independent frame per worker",
        **runtime_id,
        "source_sha256": hashes,
        **extra,
    }
    if "selected_build" in meta:
        from selected_build import verify

        cli = verify(ROOT, meta)
    (out / "manifest.json").write_text(json.dumps(meta, indent=2) + "\n")
else:
    meta = saved
if not args.summarize and cli is None and "selected_build" in meta:
    from selected_build import verify

    cli = verify(ROOT, meta)
names = ("compressed_bytes", "compress_seconds", "decompress_seconds")
rows = []
allowed = []
for lv in levels:
    for name in args.files:
        stem = f"level-{lv:02d}-{name}"
        trials = out / f"{stem}.csv"
        if not args.summarize and not (args.resume and trials.is_file()):
            pending = out / f"{stem}.partial"
            inherited = {
                k: v
                for k, v in os.environ.items()
                if not cli or not k.startswith(("BTR_", "ZSTD_", "PROFILE_", "PERF_"))
            }
            env = dict(
                inherited,
                ZSTD_LEVEL=str(lv),
                BTR_LEVEL=str(lv),
                ZSTD_WIDE="1",
                WARMUP=str(int(not args.screen)),
                TRIALS_CSV=str(pending),
                DEC_ITERS=str(args.decodes),
            )
            run_trial(
                binary.parent,
                series_lock,
                [
                    str(binary),
                    str(args.threads),
                    str(args.rounds),
                    "both",
                    str(corpus[name].path),
                ],
                env=env,
                stdout=subprocess.DEVNULL,
                cwd=ROOT,
            )
            pending.replace(trials)
        with trials.open() as f:
            data = list(csv.DictReader(f))
        first = None
        for rnd in range(1, args.rounds + 1):
            pair = [r for r in data if int(r["round"]) == rnd]
            if (
                len(pair) != 2
                or {r["order"] for r in pair} != {"0", "1"}
                or {r["codec"] for r in pair} != {"btrcmpr", "zstd"}
            ):
                sys.exit(f"invalid paired observation {rnd}: {trials}")
            who = next(r["codec"] for r in pair if r["order"] == "0")
            if who == first:
                sys.exit(f"timing order did not alternate: {trials}")
            first = who
        groups = []
        for codec in ("btrcmpr", "zstd"):
            series = sorted(
                (r for r in data if r["codec"] == codec), key=lambda r: int(r["round"])
            )
            if (
                [int(r["round"]) for r in series] != list(range(1, args.rounds + 1))
                or len(data) != 2 * args.rounds
                or any(
                    Path(r["file"]).name != name
                    or int(r["raw_bytes"]) != corpus[name].size
                    or any(
                        not math.isfinite(float(r[k])) or float(r[k]) <= 0
                        for k in names
                    )
                    for r in series
                )
                or len({r["compressed_bytes"] for r in series}) != 1
            ):
                sys.exit(f"bad or missing trials: {trials}")
            groups.append(
                [statistics.median(Decimal(r[k]) for r in series) for k in names]
            )
        a, b = groups
        flags = [u < v for u, v in zip(a, b)]
        rows.append(
            [
                lv,
                name,
                args.threads,
                args.rounds,
                int(a[0]),
                int(b[0]),
                *(round(float(b[i] / a[i]), 4) for i in (1, 2)),
                *(int(v) for v in flags),
                int(all(flags)),
            ]
        )
        ok = qualifies(groups, args.allow_decode_loss_pct)
        allowed.append(
            (
                lv,
                name,
                label(args.allow_decode_loss_pct),
                f"{100 * (b[2] / a[2] - 1):.5f}",
                int(ok),
            )
        )
        tag = (
            " at " + label(args.allow_decode_loss_pct) + "% decode allowance"
            if args.allow_decode_loss_pct
            else ""
        )
        print(
            f"{lv:2d} {name:7s}: {int(a[0]):9d}/{int(b[0]):9d} bytes  encode {b[1] / a[1]:.3f}x  decode {b[2] / a[2]:.3f}x  {'PASS' if ok else 'FAIL'}{tag}",
            flush=True,
        )
with (out / "matrix.csv").open("w") as handle:
    w = csv.writer(handle, lineterminator="\n")
    w.writerow(
        (
            "level",
            "file",
            "threads",
            "rounds",
            "btr_bytes",
            "zstd_bytes",
            "encode_speedup",
            "decode_speedup",
            "size_pass",
            "encode_pass",
            "decode_pass",
            "all_pass",
        )
    )
    w.writerows(rows)
write_csv(
    out / "matrix-decode-tolerance.csv",
    (
        "level",
        "file",
        "allow_decode_loss_pct",
        "decode_delta_pct",
        "all_pass_with_allowance",
    ),
    allowed,
)
if "selected_build" in meta:
    subprocess.run(
        [
            sys.executable,
            ROOT / "bench/tools/preset-rates.py",
            out / "matrix.csv",
            "--out",
            out / "rates.csv",
        ],
        check=True,
    )
    if cli:
        run_trial(
            cli.parent,
            series_lock,
            [
                sys.executable,
                ROOT / "bench/tools/preset-cli.py",
                out / "matrix.csv",
                "--binary",
                cli,
                "--sha256",
                meta["selected_build"]["cli_sha256"],
                "--out",
                out / "cli.csv",
            ],
            cwd=ROOT,
        )
sys.exit(0 if all(r[-1] for r in allowed) else 1)
