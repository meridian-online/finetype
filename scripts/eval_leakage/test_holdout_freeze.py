#!/usr/bin/env python3
"""End-to-end test for gittables_holdout_freeze.py.

Covers:
  - finetype-e6d ac-01: re-run on identical inputs produces byte-identical
    holdout_paths.txt (full file equality, not just set equality)
  - finetype-s16 ac-02: content-hash dedup runs before stratified sampling
  - finetype-s16 ac-03: planted-collision — duplicate parquet bytes in two
    topic dirs, only one path survives into the holdout

Builds a synthetic GitTables-shaped tree in a tempdir, freezes against
it twice, and compares the two outputs byte-for-byte.

Run standalone:
    python3 scripts/eval_leakage/test_holdout_freeze.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
FREEZE_SCRIPT = REPO_ROOT / "scripts" / "gittables_holdout_freeze.py"


def _make_synthetic_root(tmp: Path, *, plant_collision: bool = True) -> Path:
    """Create a small fixture: 8 topic clusters × 50 parquets each.

    If plant_collision, byte-identical files are written under
    cluster_a/dup.parquet AND cluster_b/dup.parquet — the dedup test
    fixture for s16 ac-03.
    """
    root = tmp / "gittables"
    n_clusters = 8
    n_per_cluster = 50
    for ci in range(n_clusters):
        cluster = root / f"cluster_{ci:02d}"
        cluster.mkdir(parents=True)
        for fi in range(n_per_cluster):
            p = cluster / f"file_{fi:03d}.parquet"
            # Each file's content is deterministic from (cluster, idx).
            payload = (f"PAR1\x00cluster={ci}\x00file={fi}\x00" * 64).encode()
            p.write_bytes(payload)

    if plant_collision:
        shared = b"PAR1\x00duplicated-across-clusters" * 128
        (root / "cluster_00" / "dup.parquet").write_bytes(shared)
        (root / "cluster_01" / "dup.parquet").write_bytes(shared)
    return root


def _read_paths(output: Path) -> list[str]:
    return [
        line for line in output.read_text().splitlines()
        if line and not line.startswith("#")
    ]


def _run_freeze(
    *, root: Path, output: Path, hash_cache: Path,
    holdout_size: int, seed: int = 20260503,
) -> None:
    cmd = [
        sys.executable, str(FREEZE_SCRIPT),
        "--root", str(root),
        "--output", str(output),
        "--hash-cache", str(hash_cache),
        "--holdout-size", str(holdout_size),
        "--seed", str(seed),
    ]
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        raise AssertionError(
            f"freeze exited {res.returncode}\nstdout:\n{res.stdout}\n"
            f"stderr:\n{res.stderr}"
        )


def test_ac01_byte_identical_rerun() -> None:
    with TemporaryDirectory() as td:
        tmp = Path(td)
        root = _make_synthetic_root(tmp)
        out1 = tmp / "out1.txt"
        out2 = tmp / "out2.txt"
        cache = tmp / "cache.tsv"

        _run_freeze(root=root, output=out1, hash_cache=cache, holdout_size=100)
        _run_freeze(root=root, output=out2, hash_cache=cache, holdout_size=100)

        b1 = out1.read_bytes()
        b2 = out2.read_bytes()
        assert b1 == b2, (
            "byte-identical re-run failed; first 200 bytes of diff:\n"
            f"out1: {b1[:200]!r}\nout2: {b2[:200]!r}"
        )


def test_ac01_byte_identical_no_cache() -> None:
    """Same idempotency without the hash cache (cold-cache reproducibility)."""
    with TemporaryDirectory() as td:
        tmp = Path(td)
        root = _make_synthetic_root(tmp)
        out1 = tmp / "out1.txt"
        out2 = tmp / "out2.txt"

        for output in (out1, out2):
            res = subprocess.run(
                [
                    sys.executable, str(FREEZE_SCRIPT),
                    "--root", str(root),
                    "--output", str(output),
                    "--no-hash-cache",
                    "--holdout-size", "100",
                ],
                capture_output=True, text=True,
            )
            assert res.returncode == 0, res.stderr

        assert out1.read_bytes() == out2.read_bytes()


def test_s16_ac03_planted_collision_dedup() -> None:
    """Two clusters share a byte-identical parquet — at most one survives."""
    with TemporaryDirectory() as td:
        tmp = Path(td)
        root = _make_synthetic_root(tmp, plant_collision=True)
        output = tmp / "holdout.txt"
        cache = tmp / "cache.tsv"

        # Force a "take everything" outcome: holdout >= total dedup pop.
        # 8 × 50 + 1 (one dup survives) = 401 unique-content files.
        _run_freeze(
            root=root, output=output, hash_cache=cache, holdout_size=401,
        )

        paths = _read_paths(output)
        dup_paths = [p for p in paths if p.endswith("dup.parquet")]
        assert len(dup_paths) == 1, (
            f"expected 1 dup.parquet to survive content-hash dedup, "
            f"got {len(dup_paths)}: {dup_paths}"
        )


def test_s16_ac02_dedup_runs_before_sampling() -> None:
    """Round-population check: dedup_population < raw_population."""
    with TemporaryDirectory() as td:
        tmp = Path(td)
        root = _make_synthetic_root(tmp, plant_collision=True)
        output = tmp / "holdout.txt"
        cache = tmp / "cache.tsv"

        _run_freeze(
            root=root, output=output, hash_cache=cache, holdout_size=50,
        )
        header = [
            line for line in output.read_text().splitlines()
            if line.startswith("# ")
        ]
        raw_line = next(l for l in header if "raw_population:" in l)
        dedup_line = next(l for l in header if "dedup_population:" in l)
        raw = int(raw_line.split(":")[1].strip())
        dedup = int(dedup_line.split(":")[1].strip())
        assert raw == 402, f"expected 402 raw, got {raw}"
        assert dedup == 401, f"expected 401 dedup, got {dedup}"


def main() -> int:
    tests = [
        ("ac-01 byte-identical re-run (with cache)", test_ac01_byte_identical_rerun),
        ("ac-01 byte-identical re-run (no cache)", test_ac01_byte_identical_no_cache),
        ("s16 ac-03 planted-collision dedup", test_s16_ac03_planted_collision_dedup),
        ("s16 ac-02 dedup before sampling", test_s16_ac02_dedup_runs_before_sampling),
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
