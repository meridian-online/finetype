#!/usr/bin/env python3
"""Per-column reject extraction for ac-08 criterion-(b) attribution.

The corpus pass (`scripts/gittables_corpus_pass.py`) stores per-FILE
reject counts in `files.parquet`, not per-column. That's enough for
the ac-07 gate but not for ac-08, which needs to attribute mechanism
tokens to specific columns.

This script re-runs profile + validate on a subset of files (intended
use: the criterion-(b)-failing subset), reads back per-column reject
counts from validate's `finetype_reject_errors` sidecar table, and
emits `eval/gittables/corpus_pass/per_column_rejects.parquet` with
rows `(file_path, column_name, reject_count)`.

Reuses `scripts/gittables_gate.py` helpers (_parquet_to_csv with the
Fix A column-name normalisation, _profile, _validate). Same per-file
cost shape as the corpus pass; running on the ~410k criterion-b
subset takes ~3-4h at jobs=16.

USAGE
    source eval/gittables/.venv/bin/activate
    python3 scripts/extract_per_column_rejects.py \\
        --paths eval/gittables/corpus_pass/criterion_b_failing_paths.txt \\
        --out   eval/gittables/corpus_pass/per_column_rejects.parquet \\
        --jobs  16
"""
from __future__ import annotations

import argparse
import concurrent.futures as cf
import json
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))
from gittables_gate import (  # noqa: E402
    _parquet_to_csv,
    _profile,
    _validate,
)

DEFAULT_DUCKDB = "duckdb"
DEFAULT_FINETYPE = "finetype"

_WORKER_FINETYPE = "finetype"
_WORKER_DUCKDB = "duckdb"


def _worker_init(finetype_bin: str, duckdb_bin: str) -> None:
    global _WORKER_FINETYPE, _WORKER_DUCKDB
    _WORKER_FINETYPE = finetype_bin
    _WORKER_DUCKDB = duckdb_bin


@dataclass
class ColumnReject:
    file_path: str
    column_name: str
    reject_count: int


def _per_column_rejects(
    db_path: Path, duckdb_bin: str
) -> list[tuple[str, int]]:
    """Read the finetype_reject_errors sidecar, group by column_name,
    return [(column_name, distinct_reject_row_count), ...]."""
    sql = (
        "SELECT column_name, COUNT(DISTINCT line) AS n "
        "FROM finetype_reject_errors "
        "GROUP BY column_name;"
    )
    res = subprocess.run(
        [duckdb_bin, "-noheader", "-csv", str(db_path), sql],
        capture_output=True, text=True, timeout=30,
    )
    if res.returncode != 0:
        # No reject sidecar (no rejects) or other failure — return empty
        return []
    out: list[tuple[str, int]] = []
    for line in res.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        # CSV with two cols. column_name may contain commas; assume no
        # commas in our gittables column names (Fix A normalised these
        # but didn't change comma handling). If this assumption breaks,
        # switch to a proper CSV reader.
        parts = line.rsplit(",", 1)
        if len(parts) != 2:
            continue
        name = parts[0].strip('"')
        try:
            count = int(parts[1])
        except ValueError:
            continue
        out.append((name, count))
    return out


def _process_one(path_str: str) -> tuple[str, list[ColumnReject], str | None]:
    """profile + validate one file, return per-column reject counts.

    Returns (file_path, rejects, error). On error, rejects is empty
    and error is the exception string."""
    parquet = Path(path_str)
    try:
        with tempfile.TemporaryDirectory(prefix="ftpcr-") as td:
            tmp = Path(td)
            csv_path = tmp / "in.csv"
            schema_path = tmp / "in.schema.json"
            db_path = tmp / "in.db"

            _parquet_to_csv(parquet, csv_path, _WORKER_DUCKDB)
            _profile(csv_path, schema_path, _WORKER_FINETYPE)
            _validate(csv_path, schema_path, db_path, _WORKER_FINETYPE)

            rejects = _per_column_rejects(db_path, _WORKER_DUCKDB)
            return (
                path_str,
                [ColumnReject(path_str, c, n) for c, n in rejects],
                None,
            )
    except Exception as exc:  # noqa: BLE001
        return (path_str, [], f"{type(exc).__name__}: {exc}"[:300])


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--paths", required=True, type=Path,
                   help="File containing input parquet paths (one per line).")
    p.add_argument("--out", required=True, type=Path,
                   help="Output parquet path.")
    p.add_argument("--jobs", type=int, default=16,
                   help="Worker process count (default 16, matches Pass A).")
    p.add_argument("--finetype-bin", default=DEFAULT_FINETYPE)
    p.add_argument("--duckdb-bin", default=DEFAULT_DUCKDB)
    p.add_argument("--log-path", type=Path, default=None,
                   help="Optional JSONL error log path.")
    args = p.parse_args()

    paths = [
        line.strip()
        for line in args.paths.read_text().splitlines()
        if line.strip() and not line.startswith("#")
    ]
    n_total = len(paths)
    if n_total == 0:
        print("error: no paths to process", file=sys.stderr)
        return 2
    print(f"per_column_rejects: {n_total} files, jobs={args.jobs}",
          file=sys.stderr)

    try:
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: pyarrow not available ({exc}).", file=sys.stderr)
        return 2

    schema = pa.schema([
        ("file_path", pa.string()),
        ("column_name", pa.string()),
        ("reject_count", pa.int64()),
    ])
    args.out.parent.mkdir(parents=True, exist_ok=True)
    writer = pq.ParquetWriter(args.out, schema, compression="snappy")
    log_fh = open(args.log_path, "w") if args.log_path else None

    file_buf: list[str] = []
    col_buf: list[str] = []
    cnt_buf: list[int] = []
    BUF = 50_000
    t0 = time.perf_counter()
    n_processed = 0
    n_errors = 0
    n_with_rejects = 0
    n_total_rows = 0

    def _flush():
        nonlocal file_buf, col_buf, cnt_buf
        if not file_buf:
            return
        table = pa.table({
            "file_path": file_buf,
            "column_name": col_buf,
            "reject_count": cnt_buf,
        }, schema=schema)
        writer.write_table(table)
        file_buf, col_buf, cnt_buf = [], [], []

    try:
        with cf.ProcessPoolExecutor(
            max_workers=args.jobs,
            initializer=_worker_init,
            initargs=(args.finetype_bin, args.duckdb_bin),
        ) as pool:
            for path_str, rejects, error in pool.map(
                _process_one, paths, chunksize=8,
            ):
                n_processed += 1
                if error:
                    n_errors += 1
                    if log_fh:
                        log_fh.write(json.dumps(
                            {"path": path_str, "error": error}
                        ) + "\n")
                if rejects:
                    n_with_rejects += 1
                    for r in rejects:
                        file_buf.append(r.file_path)
                        col_buf.append(r.column_name)
                        cnt_buf.append(r.reject_count)
                        n_total_rows += 1
                if len(file_buf) >= BUF:
                    _flush()
                if n_processed % 5000 == 0:
                    elapsed = time.perf_counter() - t0
                    rate = n_processed / elapsed
                    eta_h = (n_total - n_processed) / rate / 3600.0
                    print(
                        f"  {n_processed}/{n_total} "
                        f"({n_with_rejects} with rejects, "
                        f"{n_total_rows} rej rows, {n_errors} errors) "
                        f"@ {rate:.0f} files/s — ETA {eta_h:.2f} h",
                        file=sys.stderr,
                    )
    finally:
        _flush()
        writer.close()
        if log_fh:
            log_fh.close()

    elapsed = time.perf_counter() - t0
    print(json.dumps({
        "n_total": n_total,
        "n_processed": n_processed,
        "n_with_rejects": n_with_rejects,
        "n_total_reject_rows": n_total_rows,
        "n_errors": n_errors,
        "elapsed_seconds": round(elapsed, 1),
        "elapsed_hours": round(elapsed / 3600.0, 3),
        "output_parquet": str(args.out),
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
