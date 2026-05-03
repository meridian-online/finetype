#!/usr/bin/env python3
"""Planted-collision test for the parquet content-hash primitive.

Verifies the s16 contract:
  - identical bytes hash identically (collision)
  - one-bit differences hash differently (distinction)
  - GitTables-style duplicate-across-topic-clusters is detected
  - hash is deterministic across invocations
  - <100ms per file on M1 (synthetic 1 MiB file, generous bound)

Run standalone:
    python scripts/eval_leakage/test_content_hash.py

Run as pytest:
    pytest scripts/eval_leakage/test_content_hash.py

Exit code 0 on all-pass, 1 on any failure.
"""

from __future__ import annotations

import os
import sys
import time
from pathlib import Path
from tempfile import TemporaryDirectory

_REPO_SCRIPTS = Path(__file__).resolve().parent.parent
if str(_REPO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_REPO_SCRIPTS))

# pyright: reportMissingImports=false
from eval_leakage.content_hash import (  # noqa: E402
    CONTENT_HASH_VERSION,
    file_content_sha256,
)


def _write(p: Path, data: bytes) -> None:
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_bytes(data)


def test_identical_bytes_collide() -> None:
    with TemporaryDirectory() as td:
        a = Path(td) / "miles_per_hour" / "06-07_11.parquet"
        b = Path(td) / "inflation_rate" / "06-07_11.parquet"
        payload = b"PAR1\x00synthetic-test-payload" * 4096
        _write(a, payload)
        _write(b, payload)
        assert file_content_sha256(a) == file_content_sha256(b)


def test_one_bit_difference_distinct() -> None:
    with TemporaryDirectory() as td:
        a = Path(td) / "a.parquet"
        b = Path(td) / "b.parquet"
        payload = b"PAR1\x00" + bytes(1024)
        _write(a, payload)
        _write(b, payload[:-1] + b"\x01")
        assert file_content_sha256(a) != file_content_sha256(b)


def test_deterministic_across_invocations() -> None:
    with TemporaryDirectory() as td:
        p = Path(td) / "x.parquet"
        _write(p, b"PAR1\x00" + bytes(2048))
        h1 = file_content_sha256(p)
        h2 = file_content_sha256(p)
        h3 = file_content_sha256(p)
        assert h1 == h2 == h3


def test_missing_file_raises() -> None:
    try:
        file_content_sha256(Path("/nonexistent-path-finetype-s16-test"))
    except FileNotFoundError:
        return
    raise AssertionError("expected FileNotFoundError")


def test_performance_under_100ms_for_1mib() -> None:
    """Generous bound — real GitTables files are ~50 KB."""
    with TemporaryDirectory() as td:
        p = Path(td) / "big.parquet"
        _write(p, os.urandom(1 << 20))  # 1 MiB
        # Warm caches with one read so we measure SHA, not page-cache miss.
        file_content_sha256(p)
        t0 = time.perf_counter()
        file_content_sha256(p)
        elapsed_ms = (time.perf_counter() - t0) * 1000
        assert elapsed_ms < 100, f"hash took {elapsed_ms:.1f}ms, expected <100ms"


def test_version_pinned() -> None:
    assert CONTENT_HASH_VERSION == "1.0.0", (
        f"CONTENT_HASH_VERSION changed to {CONTENT_HASH_VERSION!r}; "
        "bumping requires regenerating any persisted content-hash artefacts "
        "(holdout, harvest_pool, coverage_log)."
    )


def main() -> int:
    print(f"content_hash test (CONTENT_HASH_VERSION = {CONTENT_HASH_VERSION})")
    tests = [
        ("identical bytes collide", test_identical_bytes_collide),
        ("one-bit difference distinct", test_one_bit_difference_distinct),
        ("deterministic across invocations", test_deterministic_across_invocations),
        ("missing file raises", test_missing_file_raises),
        ("<100ms per 1 MiB file", test_performance_under_100ms_for_1mib),
        ("version pinned", test_version_pinned),
    ]
    failed: list[str] = []
    for name, fn in tests:
        try:
            fn()
            print(f"  PASS  {name}")
        except AssertionError as exc:
            failed.append(f"{name}: {exc}")
            print(f"  FAIL  {name}: {exc}")
        except Exception as exc:  # noqa: BLE001
            failed.append(f"{name}: {type(exc).__name__}: {exc}")
            print(f"  FAIL  {name}: {type(exc).__name__}: {exc}")
    print()
    print(f"{len(tests) - len(failed)}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
