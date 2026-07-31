#!/usr/bin/env python3
"""Extract DBpedia / Schema.org annotations from every measure-half parquet.

Spec: `2026-05-20-gittables-multi-lens-diagnostic`
ac-05 (DBpedia / Schema.org annotation extraction + mapping extension).

Reads each parquet's Arrow KV-metadata footer (via pyarrow, no DuckDB
subprocess overhead) and emits per-column rows
`(file_path, column_name, dbpedia_semantic_class, dbpedia_similarity,
schema_semantic_class)` into
`eval/gittables/corpus_pass/dbpedia_annotations.parquet`.

Runs on the MEASURE half only — files where
`SHA256(file_bytes) MOD 2 == 1`. The SHA256 of each file is cached in
the output parquet for downstream reuse.

Usage (from a venv):
  source eval/gittables/.venv/bin/activate
  python3 scripts/extract_dbpedia_annotations.py --jobs 16
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import hashlib
import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_CORPUS_INDEX = REPO / "eval" / "gittables" / "corpus_paths.txt"
DEFAULT_OUT_PARQUET = (
    REPO / "eval" / "gittables" / "corpus_pass" / "dbpedia_annotations.parquet"
)
DEFAULT_OUT_COVERAGE = REPO / "eval" / "gittables" / "dbpedia_coverage.json"
MIN_CLASS_FREQ = 10  # ac-05 threshold


@dataclass(frozen=True)
class ColumnAnnotation:
    file_path: str
    file_content_sha256: str
    column_name: str
    dbpedia_semantic_class: str | None
    dbpedia_similarity: float | None
    schema_semantic_class: str | None


def _file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _read_kv_blob(path: Path) -> dict:
    """Read the `gittables` KV-metadata blob from a parquet footer."""
    import pyarrow.parquet as pq

    pf = pq.ParquetFile(path)
    kv = pf.schema_arrow.metadata or {}
    raw = kv.get(b"gittables")
    if raw is None:
        return {}
    try:
        return json.loads(raw)
    except (json.JSONDecodeError, UnicodeDecodeError):
        return {}


def _columns_from_kv(
    kv: dict, file_path: str, sha: str
) -> list[ColumnAnnotation]:
    dbpedia = kv.get("dbpedia_semantic_column_types") or {}
    schema = kv.get("schema_semantic_column_types") or {}
    dbpedia_sim = kv.get("dbpedia_semantic_similarities") or {}
    columns = set(dbpedia) | set(schema) | set(kv.get("dtypes") or {})
    out: list[ColumnAnnotation] = []
    for col in sorted(columns):
        d = dbpedia.get(col)
        s = schema.get(col)
        d_id = d.get("id") if isinstance(d, dict) else None
        s_id = s.get("id") if isinstance(s, dict) else None
        sim = dbpedia_sim.get(col)
        sim_f = float(sim) if isinstance(sim, (int, float)) else None
        out.append(ColumnAnnotation(
            file_path=file_path,
            file_content_sha256=sha,
            column_name=col,
            dbpedia_semantic_class=d_id,
            dbpedia_similarity=sim_f,
            schema_semantic_class=s_id,
        ))
    return out


def _measure_one(path_str: str) -> tuple[str, list[ColumnAnnotation], str | None]:
    """Returns (sha256, [annotations] (empty if not measure-half),
    error or None)."""
    p = Path(path_str)
    try:
        sha = _file_sha256(p)
        if int(sha, 16) % 2 != 1:
            return (sha, [], None)
        kv = _read_kv_blob(p)
        anns = _columns_from_kv(kv, str(p), sha)
        return (sha, anns, None)
    except Exception as exc:  # noqa: BLE001
        return ("", [], f"{type(exc).__name__}: {exc}"[:300])


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Extract DBpedia / Schema.org annotations from "
                    "measure-half gittables parquets (ac-05)."
    )
    parser.add_argument(
        "--corpus-index", type=Path, default=DEFAULT_CORPUS_INDEX,
    )
    parser.add_argument("--jobs", type=int, default=16)
    parser.add_argument("--max-files", type=int, default=None,
                        help="Cap for smoke testing.")
    parser.add_argument(
        "--out-parquet", type=Path, default=DEFAULT_OUT_PARQUET,
    )
    parser.add_argument(
        "--out-coverage", type=Path, default=DEFAULT_OUT_COVERAGE,
    )
    args = parser.parse_args()

    try:
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:
        print(f"error: pyarrow not available ({exc}). Activate venv: "
              "source eval/gittables/.venv/bin/activate",
              file=sys.stderr)
        return 2

    if not args.corpus_index.exists():
        print(f"error: corpus index not found: {args.corpus_index}",
              file=sys.stderr)
        return 2

    paths: list[str] = []
    with args.corpus_index.open() as fh:
        for line in fh:
            line = line.strip()
            if line and not line.startswith("#"):
                paths.append(line)
    if args.max_files is not None:
        paths = paths[: args.max_files]
    print(f"corpus_paths: {len(paths)} parquets", file=sys.stderr)

    args.out_parquet.parent.mkdir(parents=True, exist_ok=True)

    t_start = time.perf_counter()
    n_processed = 0
    n_measure = 0
    n_errors = 0
    all_columns: list[ColumnAnnotation] = []

    with cf.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        for sha, anns, err in pool.map(_measure_one, paths, chunksize=64):
            n_processed += 1
            if err:
                n_errors += 1
            elif anns:
                n_measure += 1
                all_columns.extend(anns)
            if n_processed % 10000 == 0:
                elapsed = time.perf_counter() - t_start
                rate = n_processed / elapsed if elapsed else 0
                eta = (len(paths) - n_processed) / rate if rate else 0
                print(
                    f"  {n_processed}/{len(paths)} "
                    f"({n_measure} measure-half, {n_errors} errors) "
                    f"@ {rate:.0f} files/s — ETA {eta/60:.1f} min",
                    file=sys.stderr,
                )

    elapsed = time.perf_counter() - t_start
    print(
        f"done: {n_processed} processed, {n_measure} measure-half, "
        f"{n_errors} errors, {elapsed/60:.1f} min",
        file=sys.stderr,
    )

    # Write annotations parquet.
    table = pa.table({
        "file_path": [a.file_path for a in all_columns],
        "file_content_sha256": [a.file_content_sha256 for a in all_columns],
        "column_name": [a.column_name for a in all_columns],
        "dbpedia_semantic_class": [
            a.dbpedia_semantic_class for a in all_columns
        ],
        "dbpedia_similarity": [a.dbpedia_similarity for a in all_columns],
        "schema_semantic_class": [a.schema_semantic_class for a in all_columns],
    })
    pq.write_table(table, args.out_parquet, compression="snappy")
    print(f"wrote {args.out_parquet} ({len(all_columns)} column rows)",
          file=sys.stderr)

    # Coverage report: count distinct DBpedia classes ≥ MIN_CLASS_FREQ.
    dbpedia_counts: dict[str, int] = {}
    for a in all_columns:
        if a.dbpedia_semantic_class:
            dbpedia_counts[a.dbpedia_semantic_class] = (
                dbpedia_counts.get(a.dbpedia_semantic_class, 0) + 1
            )
    ge_threshold = sorted(
        [(cls, n) for cls, n in dbpedia_counts.items() if n >= MIN_CLASS_FREQ],
        key=lambda x: (-x[1], x[0]),
    )

    coverage = {
        "n_corpus_total": len(paths),
        "n_processed": n_processed,
        "n_measure_half": n_measure,
        "n_errors": n_errors,
        "n_column_rows": len(all_columns),
        "n_distinct_dbpedia_classes": len(dbpedia_counts),
        "min_class_freq_threshold": MIN_CLASS_FREQ,
        "n_classes_at_or_above_threshold": len(ge_threshold),
        "classes_at_or_above_threshold": [
            {"class": c, "count": n} for c, n in ge_threshold
        ],
        "elapsed_minutes": round(elapsed / 60.0, 2),
    }
    args.out_coverage.write_text(json.dumps(coverage, indent=2) + "\n")
    print(f"wrote {args.out_coverage}", file=sys.stderr)
    print(
        f"summary: {len(dbpedia_counts)} distinct DBpedia classes; "
        f"{len(ge_threshold)} appear ≥ {MIN_CLASS_FREQ}× in measure half",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
