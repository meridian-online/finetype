#!/usr/bin/env python3
"""ac-13 helper — build a 100-file calibrate-half paths sample.

Scans `corpus_paths.txt` sequentially, computes SHA256 of each parquet
until 100 paths have `SHA256 % 2 == 0` (the calibrate partition).
Writes the sample list to `eval/gittables/corpus_paths_calibrate_100.txt`.

Idempotent: same corpus_paths.txt → same sample list (sequential scan,
no randomness).

USAGE
    python3 scripts/build_calibrate_paths_sample.py
"""
from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CALIBRATE_BUCKET = 0  # SHA256 % 2 == 0


def file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def sha_bucket(sha_hex: str) -> int:
    # Match scripts/gittables_corpus_pass.py:_sha_bucket byte-for-byte
    # so two paths agreeing on calibrate here will agree there too.
    return int(sha_hex, 16) % 2


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--corpus-index", type=Path,
                   default=REPO / "eval/gittables/corpus_paths.txt")
    p.add_argument("--out", type=Path,
                   default=REPO / "eval/gittables/corpus_paths_calibrate_100.txt")
    p.add_argument("--n", type=int, default=100)
    args = p.parse_args()

    if not args.corpus_index.exists():
        print(f"error: {args.corpus_index} missing", file=sys.stderr)
        return 2

    out_paths: list[str] = []
    scanned = 0
    with args.corpus_index.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            scanned += 1
            path = Path(line)
            if not path.exists():
                continue
            try:
                sha = file_sha256(path)
            except OSError:
                continue
            if sha_bucket(sha) == CALIBRATE_BUCKET:
                out_paths.append(line)
                if len(out_paths) >= args.n:
                    break

    if len(out_paths) < args.n:
        print(
            f"warning: only found {len(out_paths)} calibrate-half files "
            f"in {scanned} scanned paths (wanted {args.n})",
            file=sys.stderr,
        )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(out_paths) + "\n")
    print(
        f"wrote {len(out_paths)} calibrate-half paths to {args.out} "
        f"(scanned {scanned})",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
