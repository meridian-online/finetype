#!/usr/bin/env python3
"""Append-only log primitive tests — bead finetype-87j.

Covers:
  - ac-01: atomic append (concurrent writers don't tear writes)
  - ac-02: log_integrity_check.sh detects shrinkage (H08/H09)
  - ac-03: macOS uschg flag prevents truncation/rewrite (skipped on Linux)
  - ac-04: cycle log records pre-cycle line counts → next-cycle integrity

Run standalone:
    python3 scripts/eval_leakage/test_append_log.py
"""

from __future__ import annotations

import multiprocessing as mp
import os
import platform
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

# pyright: reportMissingImports=false
from append_log import (  # noqa: E402
    FAILURE_LOG_HEADER,
    append_jsonl_record,
    append_tsv_row,
    count_lines,
)

INTEGRITY_SCRIPT = REPO_ROOT / "scripts" / "log_integrity_check.sh"


def _writer_worker(args: tuple[str, int]) -> None:
    path_str, n = args
    path = Path(path_str)
    for i in range(n):
        append_tsv_row(
            path,
            {
                "cycle_id": f"cycle-{os.getpid()}",
                "timestamp": "2026-05-03T12:00:00Z",
                "file_path": f"/data/file_{i}.parquet",
                "file_content_sha256": f"{i:064x}",
                "column_name": f"col_{i}",
                "predicted_type": "representation.text.plain_text",
                "observed_values_sample": "a|b|c",
                "inferred_correct_type": "unknown",
                "mechanism": "value-shape",
            },
            header=FAILURE_LOG_HEADER,
        )


def test_ac01_atomic_append_no_torn_writes() -> None:
    """4 workers × 250 rows each → 1000 data rows + 1 header line, all well-formed."""
    with TemporaryDirectory() as td:
        path = Path(td) / "failure_log.tsv"
        n_workers = 4
        per_worker = 250
        with mp.Pool(n_workers) as pool:
            pool.map(
                _writer_worker,
                [(str(path), per_worker)] * n_workers,
            )
        text = path.read_text()
        lines = text.splitlines()
        assert len(lines) == 1 + n_workers * per_worker, (
            f"expected {1 + n_workers * per_worker} lines, got {len(lines)}"
        )
        # Header check
        assert lines[0].split("\t") == list(FAILURE_LOG_HEADER)
        # Every data row must have the right column count
        n_cols = len(FAILURE_LOG_HEADER)
        for i, line in enumerate(lines[1:], start=2):
            cells = line.split("\t")
            assert len(cells) == n_cols, (
                f"line {i} has {len(cells)} cells, expected {n_cols}: {line!r}"
            )


def test_ac02_integrity_check_detects_shrinkage() -> None:
    """Direct-mode integrity check fails when current < baseline."""
    with TemporaryDirectory() as td:
        path = Path(td) / "log.tsv"
        path.write_text("a\nb\nc\n")  # 3 lines

        # baseline 3, current 3 → ok
        res = subprocess.run(
            [str(INTEGRITY_SCRIPT), "--baseline", "3", "--file", str(path)],
            capture_output=True, text=True,
        )
        assert res.returncode == 0, f"expected 0, got {res.returncode}: {res.stderr}"

        # baseline 3, current 2 → halt
        path.write_text("a\nb\n")
        res = subprocess.run(
            [str(INTEGRITY_SCRIPT), "--baseline", "3", "--file", str(path)],
            capture_output=True, text=True,
        )
        assert res.returncode == 1, f"expected 1 (halt), got {res.returncode}"
        assert "HALT" in res.stderr


def test_ac02_integrity_check_cycle_mode() -> None:
    """Cycle mode: derives baseline from cycle_log.jsonl's last line."""
    with TemporaryDirectory() as td:
        # Build a fake repo layout the script expects.
        eval_dir = Path(td) / "eval" / "gittables"
        eval_dir.mkdir(parents=True)

        failure_log = eval_dir / "failure_log.tsv"
        coverage_log = eval_dir / "working_slice_coverage.tsv"
        cycle_log = eval_dir / "cycle_log.jsonl"

        # Build state: previous cycle ended with 5 failure lines, 7 coverage lines.
        failure_log.write_text("h\n" + "x\n" * 4)  # 5 lines
        coverage_log.write_text("h\n" + "x\n" * 6)  # 7 lines
        append_jsonl_record(
            cycle_log,
            {
                "cycle_id": "abc",
                "failure_log_lines_after": 5,
                "coverage_log_lines_after": 7,
            },
        )

        env = {**os.environ, "FINETYPE_REPO_ROOT": str(td)}
        res = subprocess.run(
            [str(INTEGRITY_SCRIPT)],
            env=env,
            capture_output=True, text=True,
        )
        assert res.returncode == 0, (
            f"expected 0, got {res.returncode}\nstderr:\n{res.stderr}\n"
            f"stdout:\n{res.stdout}"
        )

        # Now corrupt the failure_log (truncate to 3 lines).
        failure_log.write_text("h\n" + "x\n" * 2)  # 3 lines
        res = subprocess.run(
            [str(INTEGRITY_SCRIPT)],
            env=env,
            capture_output=True, text=True,
        )
        assert res.returncode == 1, f"expected 1 (halt), got {res.returncode}"
        assert "H08" in res.stderr


def test_ac04_count_lines_helper() -> None:
    """count_lines() returns 0 on missing, n on existing — used by preamble."""
    with TemporaryDirectory() as td:
        path = Path(td) / "log.tsv"
        assert count_lines(path) == 0
        path.write_text("h\na\nb\n")
        assert count_lines(path) == 3


def test_ac03_uschg_flag_prevents_overwrite() -> None:
    """macOS user-immutable flag blocks truncation/rewrite.

    On Linux, chflags doesn't exist — the equivalent (chattr +a) needs
    root, and the test would have to skip anyway, so we just check the
    macOS path. Testing an externally-applied flag verifies the
    contract's protection — not the writer's behaviour.
    """
    if platform.system() != "Darwin":
        print("  SKIP  (non-macOS)")
        return

    with TemporaryDirectory() as td:
        path = Path(td) / "locked.tsv"
        path.write_text("original\n")

        # Apply uschg flag.
        try:
            subprocess.run(
                ["chflags", "uchg", str(path)], check=True,
                capture_output=True, text=True,
            )
        except subprocess.CalledProcessError as exc:
            # Permission denied (sandbox) — skip rather than fail.
            print(f"  SKIP  (chflags failed: {exc.stderr})")
            return

        try:
            # Truncation must fail.
            try:
                with open(path, "w") as fh:
                    fh.write("rewritten\n")
            except PermissionError:
                # Expected.
                content_after = path.read_text()
                assert content_after == "original\n", (
                    f"flag failed to protect: content is {content_after!r}"
                )
                return
            # If the open + write succeeded, the flag isn't doing its job.
            content_after = path.read_text()
            if content_after != "original\n":
                raise AssertionError(
                    f"uschg failed to prevent truncation; content={content_after!r}"
                )
        finally:
            # Always drop the flag so the tempdir cleanup works.
            subprocess.run(
                ["chflags", "nouchg", str(path)], check=False,
                capture_output=True,
            )


def main() -> int:
    tests = [
        ("ac-01 atomic append (no torn writes)", test_ac01_atomic_append_no_torn_writes),
        ("ac-02 integrity check direct-mode shrinkage", test_ac02_integrity_check_detects_shrinkage),
        ("ac-02 integrity check cycle mode", test_ac02_integrity_check_cycle_mode),
        ("ac-04 count_lines helper", test_ac04_count_lines_helper),
        ("ac-03 uschg blocks rewrite (macOS)", test_ac03_uschg_flag_prevents_overwrite),
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
