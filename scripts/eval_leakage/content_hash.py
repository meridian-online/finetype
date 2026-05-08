"""Whole-file content hashing for parquet-level leakage prevention.

Extends MADR 0056's row-level normaliser-hash with a complementary
file-level hash. Where row hashing prevents a single (header, value)
tuple from leaking into both train and eval, content hashing prevents
the same parquet file from appearing on both sides of the holdout /
working-slice / harvest partition.

This is required by autonomy contract §2 invariants — GitTables
duplicates the same parquet across topic-cluster directories
(e.g. `miles_per_hour/06-07_11.parquet` and
`inflation_rate/06-07_11.parquet` share content). A path-based filter
would let the same file enter both holdout and working slice.

Spec:

    file_content_sha256 = sha256(<entire file bytes>).hexdigest()

The hash is computed over the file's bytes as written on disk (the
parquet container, not the decompressed Arrow table). Two parquet
files written by different writers from identical Arrow tables MAY
hash differently — this is acceptable. The threat model is verbatim
duplication (cp / hardlink across topic-clusters), which is what
GitTables exhibits.

Performance: chunked read keeps memory bounded; SHA256 is implemented
in OpenSSL on macOS, ~500 MB/s. <100ms per file at typical GitTables
parquet sizes (~50 KB) is achieved trivially.

See:
    orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml §2 invariants
    .orbit/choices/0056-train-eval-leakage-prevention.md
    bead finetype-s16
"""

from __future__ import annotations

import hashlib
from pathlib import Path

__all__ = [
    "file_content_sha256",
    "CONTENT_HASH_VERSION",
]


# Bumped when the spec changes. Consumers (holdout freeze, harvest
# pool, coverage log) record this alongside hashes so a future spec
# change is detectable.
CONTENT_HASH_VERSION = "1.0.0"


# 1 MiB matches OS page-cache friendly read sizes; tuning above this
# ceases to matter for typical GitTables file sizes (~50 KB) but
# protects callers passing larger paths.
_CHUNK_BYTES = 1 << 20


def file_content_sha256(path: str | Path) -> str:
    """Hex-digested SHA256 over the byte contents of a file.

    Raises FileNotFoundError if the path does not exist (no silent
    empty-string return — leakage tooling fails loud, never silent).
    """
    p = Path(path)
    h = hashlib.sha256()
    with open(p, "rb") as fh:
        while True:
            chunk = fh.read(_CHUNK_BYTES)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()
