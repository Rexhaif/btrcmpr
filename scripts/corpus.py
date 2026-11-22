"""Published inputs shared by the downloader, comparator and standalone checks."""

import argparse
import bz2
import hashlib
import json
import lzma
import shutil
import subprocess
import tempfile
import zipfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Input:
    path: Path
    size: int
    digest: str
    url: str


def original(name, size, digest, url):
    return Input(Path("bench/data") / name, size, digest, url)


ORIGINAL = {
    "enwik8": original(
        "enwik8",
        100000000,
        "2b49720ec4d78c3c9fabaee6e4179a5e997302b3a70029f30f2d582218c024a8",
        "https://mattmahoney.net/dc/enwik8.zip",
    ),
    "x-ray": original(
        "x-ray",
        8474240,
        "7de9fce1405dc44ae5e6813ed21cd5751e761bd4265655a005d39b9685d1c9ad",
        "https://sun.aei.polsl.pl/~sdeor/corpus/x-ray.bz2",
    ),
    "mozilla": original(
        "mozilla",
        51220480,
        "657fc3764b0c75ac9de9623125705831ebbfbe08fed248df73bc2dc66e2a963b",
        "https://sun.aei.polsl.pl/~sdeor/corpus/mozilla.bz2",
    ),
}
_manifest = json.loads((ROOT / "bench/corpora/squash-extra.json").read_text())
EXTRA = {
    r["file"]: Input(
        Path(_manifest["data_directory"]) / r["file"],
        r["raw_bytes"],
        r["raw_sha256"],
        r["source_url"],
    )
    for r in _manifest["files"]
}
CORPORA = {"original": ORIGINAL, "extra": EXTRA}
INPUTS = ORIGINAL | EXTRA


def matches(path, inp):
    if not path.is_file() or path.stat().st_size != inp.size:
        return False
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest() == inp.digest


def fetch(inp):
    path = ROOT / inp.path
    if matches(path, inp):
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=path.parent) as work:
        archive, raw = Path(work) / "archive", Path(work) / "unpacked"
        subprocess.run(
            ["curl", "-fsSL", "--retry", "3", "-o", archive, inp.url], check=True
        )
        with raw.open("wb") as dest:
            if inp.url.endswith(".zip"):
                with zipfile.ZipFile(archive) as z, z.open(path.name) as src:
                    shutil.copyfileobj(src, dest)
            else:
                opener = lzma.open if inp.url.endswith(".xz") else bz2.open
                with opener(archive, "rb") as src:
                    shutil.copyfileobj(src, dest)
        if not matches(raw, inp):
            raise ValueError(f"size or SHA-256 mismatch: {path.name}")
        raw.replace(path)


if __name__ == "__main__":
    p = argparse.ArgumentParser(description="Fetch and verify Squash benchmark inputs.")
    p.add_argument(
        "--extra", action="store_true", help="fetch the other 25 Squash files"
    )
    a = p.parse_args()
    corpus = EXTRA if a.extra else ORIGINAL
    for inp in corpus.values():
        fetch(inp)
    print(f"Verified {len(corpus)} inputs.")
