"""Run the independent input schedule used to profile the native production compressor."""

import hashlib
import subprocess
from pathlib import Path


def digest(p):
    with Path(p).open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def train(runner, training, raw, profdata, result, env, cwd):
    raw.mkdir(parents=True, exist_ok=True)
    if any(raw.glob("*.profraw")):
        raise ValueError("profile counter directory must be fresh")
    e = dict(env, LLVM_PROFILE_FILE=str(raw / "%p-%m.profraw"), ZSTD_WIDE="1")
    for name in ("manuals", "modules", "executables", "samples", "records.tar"):
        for level in (1, 2, 3, 6, 10, 17, 22):
            src = training / name
            if level >= 6:
                clip = training / (name + ".small")
                clip.write_bytes(src.read_bytes()[: 512 * 1024])
                src = clip
            for direction, reps in [
                ("c", 4 if level == 1 else 1),
                ("d", 6 if level == 1 else 1),
            ]:
                p = subprocess.run(
                    [runner, direction, "ours", src, "4", str(reps)],
                    cwd=cwd,
                    env=dict(e, BTR_LEVEL=str(level)),
                    capture_output=True,
                    check=False,
                )
                if p.returncode:
                    raise RuntimeError(p.stderr.decode(errors="replace")[-1800:])
    parts = sorted(raw.glob("*.profraw"))
    if not parts:
        raise RuntimeError("instrumented program produced no counters")
    subprocess.run(
        [profdata, "merge", "-o", result, *parts], cwd=cwd, env=e, check=True
    )
    return result
