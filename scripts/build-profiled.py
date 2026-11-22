#!/usr/bin/env python3
"""Build the scratch ARM codec using thin LTO and an independently trained LLVM profile."""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

root = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(root / "bench/tools"))
from profile_training import digest, train

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--workspace", type=Path, default=root, help="Rust source directory")
p.add_argument(
    "--target-dir", type=Path, help="destination for the optimized cargo artifacts"
)
p.add_argument(
    "--profile-data",
    type=Path,
    help="use an existing llvm-profdata file rather than train",
)
p.add_argument("--profile-sha256", help="verify the supplied profile")
p.add_argument(
    "--comparators",
    action="store_true",
    help="also build the separate benchmark executables that link system libzstd",
)
p.add_argument(
    "--selected",
    action="store_true",
    help="build the current combined codec and its comparison tools with the measured settings",
)
a = p.parse_args()
if a.selected:
    a.comparators = True
src = a.workspace.resolve()
target = (a.target_dir or src / "target/profiled").resolve()
target.mkdir(exist_ok=True, parents=True)
if a.profile_sha256 and not a.profile_data:
    p.error("--profile-sha256 requires --profile-data")
env = {
    k: v
    for k, v in os.environ.items()
    if not k.startswith(("BTR_", "PERF_", "PROFILE_", "CARGO_PROFILE_", "ZSTD_"))
    and k
    not in (
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "LLVM_PROFILE_FILE",
        "CARGO_TARGET_DIR",
    )
}
env.update(
    LC_ALL="C",
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS="1",
    CARGO_PROFILE_RELEASE_LTO="thin",
)


def run(*cmd, e=env):
    return subprocess.check_output(cmd, env=e, cwd=src, text=True)


def build(folder, flag, targets):
    e = dict(
        env,
        CARGO_TARGET_DIR=str(folder),
        CARGO_ENCODED_RUSTFLAGS=f"-Ctarget-cpu=native\x1f{flag}",
    )
    subprocess.run(
        ["cargo", "build", "--offline", "--locked", "--release", *targets],
        env=e,
        cwd=src,
        check=True,
    )
    return e


rustc = run("rustc", "-Vv")
host = re.search(r"host: (\S+)", rustc)[1]
sources = {
    str(p.relative_to(src)): digest(p) for p in sorted((src / "src").rglob("*.rs"))
}
training_meta = None
if a.profile_data:
    model = a.profile_data.resolve()
    if not model.is_file() or a.profile_sha256 and digest(model) != a.profile_sha256:
        p.error("profile file is missing or its digest differs")
else:
    tools = (
        Path(run("rustc", "--print", "sysroot").strip())
        / "lib/rustlib"
        / host
        / "bin/llvm-profdata"
    )
    if not tools.is_file():
        p.error("run: rustup component add llvm-tools-preview")
    with tempfile.TemporaryDirectory(dir=target, prefix="train-") as t:
        tmp = Path(t).resolve()
        corpus = tmp / "inputs"
        raw = tmp / "raw"
        raw.mkdir()
        subprocess.run(
            [sys.executable, root / "bench/tools/codegen-training.py", "--out", corpus],
            env=env,
            check=True,
        )
        trainenv = build(
            tmp / "instrumented",
            "-Cprofile-generate=" + str(raw),
            ["--features", "baseline", "--example", "profile"],
        )
        pending = tmp / "native.profdata"
        train(
            tmp / "instrumented/release/examples/profile",
            corpus,
            raw,
            tools,
            pending,
            trainenv,
            src,
        )
        training_meta = json.loads((corpus / "training.json").read_text())
        # A profile's digest is part of its name, including when Rust changes.
        model = target / ("native-" + digest(pending) + ".profdata")
        pending.replace(model)
targets = (
    [
        "--features",
        "baseline",
        "--example",
        "profile",
        "--bin",
        "compare",
        "--bin",
        "btrcmpr",
    ]
    if a.comparators
    else ["--bin", "btrcmpr"]
)
build(target, "-Cprofile-use=" + str(model), targets)
cli = target / "release/btrcmpr"
if "libzstd" in run("ldd", str(cli)):
    raise ValueError("standalone CLI unexpectedly links libzstd")
spec = {
    "rustc": rustc,
    "workers_used_for_training": 4,
    "settings": {"lto": "thin", "codegen_units": 1, "target_cpu": "native"},
    "profile_data_sha256": digest(model),
    "training": training_meta,
    "features": targets,
    "source_sha256": sources,
    "cli_sha256": digest(cli),
}
if a.comparators:
    spec.update(
        comparator_sha256=digest(target / "release/compare"),
        profile_runner_sha256=digest(target / "release/examples/profile"),
    )
(target / "build-profiled.json").write_text(json.dumps(spec, indent=2) + "\n")
print("Standalone codec:", cli)
print("Build manifest:", target / "build-profiled.json")
