"""Cache the profiled binaries by SHA-256."""

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def sha(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def freeze(root, source_hashes):
    target = root / "target/profiled"
    manifest = target / "build-profiled.json"
    if not manifest.is_file():
        subprocess.run(
            [sys.executable, root / "scripts/build-profiled.py", "--selected"],
            cwd=root,
            stdout=sys.stderr,
            check=True,
        )
    build = json.loads(manifest.read_text())
    if (
        build.get("source_sha256") != source_hashes
        or build.get("features")
        != [
            "--features",
            "baseline",
            "--example",
            "profile",
            "--bin",
            "compare",
            "--bin",
            "btrcmpr",
        ]
        or build.get("settings")
        != {"lto": "thin", "codegen_units": 1, "target_cpu": "native"}
        or not re.fullmatch("[a-f0-9]{64}", build.get("profile_data_sha256", ""))
    ):
        raise SystemExit(
            "profiled build mismatch; rebuild with: python3 scripts/build-profiled.py --selected"
        )
    cache = root / "bench/local/preset-binaries"
    cache.mkdir(parents=True, exist_ok=True)
    for name, field, prefix in [
        ("compare", "comparator_sha256", ""),
        ("btrcmpr", "cli_sha256", "cli-"),
        ("examples/profile", "profile_runner_sha256", "profile-"),
    ]:
        digest, src = build[field], target / "release" / name
        if not re.fullmatch("[a-f0-9]{64}", digest) or sha(src) != digest:
            raise SystemExit(f"{name}: build manifest hash mismatch")
        dst = cache / (prefix + digest)
        if not dst.exists():
            with tempfile.NamedTemporaryFile(dir=cache, delete=False) as tmp:
                staged = Path(tmp.name)
            try:
                shutil.copy2(src, staged)
                if sha(staged) != digest:
                    raise ValueError("compiler output changed while freezing it")
                os.replace(staged, dst)
            finally:
                staged.unlink(missing_ok=True)
        if sha(dst) != digest:
            raise SystemExit(f"{name}: cached binary hash mismatch")
    stamp = {
        k: build[k]
        for k in (
            "comparator_sha256",
            "cli_sha256",
            "profile_runner_sha256",
            "profile_data_sha256",
            "settings",
        )
    }
    return cache / stamp["comparator_sha256"], stamp


def verify(root, meta):
    stamp = meta["selected_build"]
    if stamp["comparator_sha256"] != meta["benchmark_sha256"] or not re.fullmatch(
        "[a-f0-9]{64}", stamp["cli_sha256"]
    ):
        raise ValueError("binary hashes disagree with the timing manifest")
    cli = root / "bench/local/preset-binaries" / ("cli-" + stamp["cli_sha256"])
    if sha(cli) != stamp["cli_sha256"] or "libzstd" in subprocess.check_output(
        ["ldd", cli], text=True
    ):
        raise ValueError("CLI hash mismatch or unexpected libzstd dependency")
    from corpus import INPUTS

    if any(meta["input_sha256"].get(n) != INPUTS[n].digest for n in meta["files"]):
        raise ValueError("input hash mismatch; fetch the corpus again")
    return cli


if __name__ == "__main__":
    import argparse

    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--profile",
        action="store_true",
        required=True,
        help="print the path to the perf driver",
    )
    p.parse_args()
    root = Path(__file__).resolve().parents[1]
    hashes = {str(f.relative_to(root)): sha(f) for f in (root / "src").rglob("*.rs")}
    bench, stamp = freeze(root, hashes)
    print(bench.parent / ("profile-" + stamp["profile_runner_sha256"]))
