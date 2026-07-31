#!/usr/bin/env python3
"""Full-corpus pass for the gittables multi-lens diagnostic.

Spec: `2026-05-20-gittables-multi-lens-diagnostic`
ac-04 (corpus index + dry-run runtime budget); ac-06 (full execute).

Extends `scripts/gittables_gate.py`'s per-file profile + validate
pipeline with one new stage — YDF lens inference per column
(DBpedia annotations come from `dbpedia_annotations.parquet` produced
by ac-05; ac-06 leaves them for downstream DuckDB JOINs rather than
re-extracting per file). Runs on the MEASURE half of the corpus
(`file_content_sha256 MOD 2 == 1`), enumerated from
`eval/gittables/corpus_paths.txt`.

Modes:
  --dry-run   Times the per-file pipeline on a sample (--max-files
              measure-half hits), projects linearly to full corpus,
              emits `eval/gittables/corpus_paths_dryrun.json`.
              No Parquet outputs.
  --execute   Full execute (ac-06). Writes
              `eval/gittables/corpus_pass/files.parquet` and
              `eval/gittables/corpus_pass/columns.parquet` incrementally
              via pyarrow ParquetWriter. Continues on per-file errors;
              records errors in `files.parquet.error`. Designed to be
              resumable via `--resume` (checkpoints which file_paths
              are already in files.parquet and skips them).

Usage (from a venv with ydf + pyarrow):
  source eval/gittables/.venv/bin/activate
  # Smoke test (100 files):
  python3 scripts/gittables_corpus_pass.py --jobs 8 --max-files 100 --execute \
      --out-dir /tmp/corpus-pass-smoke
  # Real run:
  python3 scripts/gittables_corpus_pass.py --jobs 16 --execute
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import hashlib
import json
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))
from gittables_gate import (  # noqa: E402
    _column_types,
    _distinct_rejected_rows_in,
    _parquet_to_csv,
    _profile,
    _validate,
)
from train_ydf import _features  # noqa: E402

DEFAULT_CORPUS_INDEX = REPO / "eval" / "gittables" / "corpus_paths.txt"
DEFAULT_YDF_MODEL = REPO / "eval" / "gittables" / "models" / "ydf.bin"
DEFAULT_YDF_VOCAB = REPO / "eval" / "gittables" / "models" / "ydf_tfidf_vocab.json"
DEFAULT_DUCKDB = "duckdb"
DEFAULT_FINETYPE = "finetype"
DEFAULT_OUT_DIR = REPO / "eval" / "gittables"


@dataclass
class FileTimings:
    path: str
    n_cols: int = 0
    sha256_s: float = 0.0
    parquet_to_csv_s: float = 0.0
    profile_s: float = 0.0
    validate_s: float = 0.0
    ydf_s: float = 0.0
    dbpedia_kv_s: float = 0.0
    total_s: float = 0.0
    in_measure_half: bool = False
    error: str | None = None


@dataclass
class FileResult:
    file_path: str
    file_content_sha256: str
    n_cols: int = 0
    non_trivial_cols: int = 0
    total_rows: int = 0
    rejects_non_trivial: int = 0
    elapsed_s: float = 0.0
    error: str | None = None


@dataclass
class ColumnResult:
    file_path: str
    file_content_sha256: str
    column_name: str
    sense_prediction: str | None = None
    sense_confidence: float | None = None
    ydf_prediction: str | None = None
    ydf_confidence: float | None = None
    sample_values_truncated: str = ""
    is_trivial: bool = False


TRIVIAL_TYPES: frozenset[str] = frozenset({
    "representation.text.plain_text",
    "representation.numeric.decimal_number",
})
SAMPLE_VALUE_MAX_LEN = 64
SAMPLE_JOIN_SEP = "│"  # U+2502 — matches labelled_eval.tsv

# Worker globals, populated by `_worker_init` in each process.
# Pass A (--execute) does NOT load the YDF model in workers — that was
# the OOM root cause. YDF inference happens later in a single
# sequential pass via `--fill-ydf`.
_WORKER_FINETYPE = "finetype"
_WORKER_DUCKDB = "duckdb"
# Partition selector — which SHA bucket to process.
#   "measure"   → SHA bucket == 1 (the measure half, default for ac-06)
#   "calibrate" → SHA bucket == 0 (the calibrate half, used by ac-13's
#                  reproducibility sub-pass to keep measure-half
#                  hygiene from debugging traffic)
_WORKER_PARTITION = "measure"


def _worker_init(
    finetype_bin: str,
    duckdb_bin: str,
    partition: str = "measure",
) -> None:
    global _WORKER_FINETYPE, _WORKER_DUCKDB, _WORKER_PARTITION
    _WORKER_FINETYPE = finetype_bin
    _WORKER_DUCKDB = duckdb_bin
    _WORKER_PARTITION = partition


def _ydf_predict_with_confidence(
    column_samples: dict[str, list[str]],
    ydf_model,
    vocab: list[str],
) -> dict[str, tuple[str, float]]:
    """Like `_ydf_predict_columns` but also returns top-1 probability."""
    import pandas as pd

    if not column_samples:
        return {}
    feat_rows = []
    col_order = list(column_samples.keys())
    for col in col_order:
        feat_rows.append(_features(column_samples[col], vocab))
    df = pd.DataFrame(feat_rows)
    preds = ydf_model.predict(df)
    label_classes = list(ydf_model.label_classes())
    out: dict[str, tuple[str, float]] = {}
    for i, col in enumerate(col_order):
        probs = preds[i]
        top_idx = max(range(len(probs)), key=lambda k: probs[k])
        out[col] = (label_classes[top_idx], float(probs[top_idx]))
    return out


def _truncate_value(v: str) -> str:
    if len(v) <= SAMPLE_VALUE_MAX_LEN:
        return v
    return v[:SAMPLE_VALUE_MAX_LEN]


def _execute_one(path_str: str) -> tuple[FileResult, list[ColumnResult]]:
    """Per-file execute: SHA -> partition filter -> profile + validate
    -> YDF per column. Skipped files return empty column list and the
    file row carries `in_measure_half == False` (signalled by an empty
    `file_content_sha256` to keep the schema flat).

    The partition the worker accepts is set by `_worker_init` via the
    `_WORKER_PARTITION` global — "measure" (SHA % 2 == 1, default
    ac-06 behaviour) or "calibrate" (SHA % 2 == 0, used by ac-13's
    reproducibility sub-pass on the calibrate half)."""
    parquet = Path(path_str)
    t0 = time.perf_counter()
    fr = FileResult(file_path=path_str, file_content_sha256="")
    cols: list[ColumnResult] = []
    try:
        sha = _file_sha256(parquet)
        wanted_bucket = 0 if _WORKER_PARTITION == "calibrate" else 1
        if _sha_bucket(sha) != wanted_bucket:
            fr.elapsed_s = time.perf_counter() - t0
            return (fr, cols)
        fr.file_content_sha256 = sha

        with tempfile.TemporaryDirectory(prefix="ftcp-exec-") as td:
            tmp = Path(td)
            csv_path = tmp / "in.csv"
            schema_path = tmp / "in.schema.json"
            db_path = tmp / "in.db"

            _parquet_to_csv(parquet, csv_path, _WORKER_DUCKDB)
            _profile(csv_path, schema_path, _WORKER_FINETYPE)
            # Validate consumes the corpus pass's CSV (column names
            # already normalised in _parquet_to_csv). v0.6.20's
            # parquet-direct path also works, but at jobs=16 it adds a
            # duckdb spawn per file inside validate, which raises peak
            # concurrent duckdb processes high enough that macOS
            # posix_spawnp intermittently fails with ENOENT (~2% of
            # files in run-3). Staying on CSV here keeps the spawn
            # count at run-1 baseline.
            summary = _validate(
                csv_path, schema_path, db_path, _WORKER_FINETYPE,
            )

            col_types = _column_types(schema_path)
            fr.n_cols = len(col_types)
            non_trivial_cols = {
                c for c, t in col_types.items()
                if t and t not in TRIVIAL_TYPES
            }
            fr.non_trivial_cols = len(non_trivial_cols)
            fr.total_rows = int(summary.get("total_rows", 0))
            fr.rejects_non_trivial = (
                _distinct_rejected_rows_in(
                    db_path, _WORKER_DUCKDB, non_trivial_cols,
                ) if fr.n_cols else 0
            )

            col_samples = _read_column_samples(csv_path, _WORKER_DUCKDB)

            for col_name, sense_label in col_types.items():
                samples = col_samples.get(col_name, [])
                truncated = SAMPLE_JOIN_SEP.join(
                    _truncate_value(v) for v in samples
                )
                cols.append(ColumnResult(
                    file_path=path_str,
                    file_content_sha256=sha,
                    column_name=col_name,
                    sense_prediction=sense_label or None,
                    sense_confidence=None,  # JSON schema doesn't carry one
                    ydf_prediction=None,    # filled by `--fill-ydf` Pass B
                    ydf_confidence=None,
                    sample_values_truncated=truncated,
                    is_trivial=(sense_label in TRIVIAL_TYPES),
                ))

        fr.elapsed_s = time.perf_counter() - t0
        return (fr, cols)
    except Exception as exc:  # noqa: BLE001
        fr.error = f"{type(exc).__name__}: {exc}"[:300]
        fr.elapsed_s = time.perf_counter() - t0
        return (fr, cols)


def _file_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _sha_bucket(sha_hex: str) -> int:
    return int(sha_hex, 16) % 2


def _extract_dbpedia_kv(parquet: Path, duckdb_bin: str) -> dict:
    sql = (
        "SELECT decode(value) FROM parquet_kv_metadata("
        f"'{str(parquet).replace(chr(39), chr(39) * 2)}'"
        ") WHERE decode(key)='gittables';"
    )
    res = subprocess.run(
        [duckdb_bin, "-noheader", "-list", "-c", sql],
        capture_output=True, text=True, timeout=30,
    )
    if res.returncode != 0 or not res.stdout.strip():
        return {}
    try:
        return json.loads(res.stdout.strip())
    except json.JSONDecodeError:
        return {}


def _ydf_predict_columns(
    column_samples: dict[str, list[str]],
    ydf_model,  # ydf.Model — typed loose to avoid import-time dep
    vocab: list[str],
) -> dict[str, str]:
    """Returns {column_name: top1_label} for every column."""
    import pandas as pd  # local — only needed in pipeline mode

    if not column_samples:
        return {}
    feat_rows = []
    col_order = list(column_samples.keys())
    for col in col_order:
        feat_rows.append(_features(column_samples[col], vocab))
    df = pd.DataFrame(feat_rows)
    preds = ydf_model.predict(df)
    label_classes = list(ydf_model.label_classes())
    out: dict[str, str] = {}
    for i, col in enumerate(col_order):
        probs = preds[i]
        top_idx = max(range(len(probs)), key=lambda k: probs[k])
        out[col] = label_classes[top_idx]
    return out


def _read_column_samples(csv_path: Path, duckdb_bin: str) -> dict[str, list[str]]:
    """Reads up to 8 sample values per column. Mirrors the
    OBSERVED_SAMPLE_LIMIT used by failure_log + labelled_eval.

    Implementation: Python csv reader on the first 9 lines of the file
    (header + 8 data rows). No DuckDB subprocess — saves ~50–100ms per
    file at corpus scale. duckdb_bin retained for API compat with
    `_measure_one`'s signature.
    """
    _ = duckdb_bin
    import csv as _csv
    by_col: dict[str, list[str]] = {}
    try:
        with csv_path.open(newline="") as fh:
            reader = _csv.reader(fh)
            try:
                header = next(reader)
            except StopIteration:
                return {}
            by_col = {c: [] for c in header}
            for n, row in enumerate(reader):
                if n >= 8:
                    break
                for i, c in enumerate(header):
                    if i < len(row) and row[i] != "":
                        by_col[c].append(row[i])
    except Exception:  # noqa: BLE001
        return {}
    return by_col


def _measure_one(
    parquet: Path,
    *,
    finetype_bin: str,
    duckdb_bin: str,
    ydf_model,
    ydf_vocab: list[str],
    measure_only: bool,
) -> FileTimings:
    t0 = time.perf_counter()
    out = FileTimings(path=str(parquet))
    try:
        t_sha = time.perf_counter()
        sha = _file_sha256(parquet)
        out.sha256_s = time.perf_counter() - t_sha
        out.in_measure_half = (_sha_bucket(sha) == 1)
        if measure_only and not out.in_measure_half:
            out.total_s = time.perf_counter() - t0
            return out

        with tempfile.TemporaryDirectory(prefix="ftcp-") as td:
            tmp = Path(td)
            csv_path = tmp / "in.csv"
            schema_path = tmp / "in.schema.json"
            db_path = tmp / "in.db"

            t = time.perf_counter()
            _parquet_to_csv(parquet, csv_path, duckdb_bin)
            out.parquet_to_csv_s = time.perf_counter() - t

            t = time.perf_counter()
            _profile(csv_path, schema_path, finetype_bin)
            out.profile_s = time.perf_counter() - t

            # Validate consumes the corpus pass's CSV; names already
            # normalised in _parquet_to_csv (see run-3 regression notes
            # in _execute_one for why we don't use validate's parquet
            # path here).
            t = time.perf_counter()
            summary = _validate(csv_path, schema_path, db_path, finetype_bin)
            out.validate_s = time.perf_counter() - t
            _ = summary  # full pass would consume this for the gate

            col_types = _column_types(schema_path)
            out.n_cols = len(col_types)
            _ = _distinct_rejected_rows_in  # silence unused; ac-06 wires this in

            if ydf_model is not None:
                t = time.perf_counter()
                col_samples = _read_column_samples(csv_path, duckdb_bin)
                _ = _ydf_predict_columns(col_samples, ydf_model, ydf_vocab)
                out.ydf_s = time.perf_counter() - t

            t = time.perf_counter()
            _ = _extract_dbpedia_kv(parquet, duckdb_bin)
            out.dbpedia_kv_s = time.perf_counter() - t

        out.total_s = time.perf_counter() - t0
        return out
    except Exception as exc:  # noqa: BLE001
        out.error = f"{type(exc).__name__}: {exc}"[:300]
        out.total_s = time.perf_counter() - t0
        return out


def _run_execute(args, paths: list[Path]) -> int:
    """ac-06 full execute. Writes files.parquet + columns.parquet
    incrementally via pyarrow ParquetWriter. Supports resume by
    checkpointing already-completed file_paths read from an existing
    files.parquet under --out-dir."""
    try:
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:
        print(f"error: pyarrow not available ({exc}).", file=sys.stderr)
        return 2

    out_dir = args.out_dir / "corpus_pass"
    out_dir.mkdir(parents=True, exist_ok=True)
    files_path = out_dir / "files.parquet"
    cols_path = out_dir / "columns.parquet"
    log_path = out_dir / "execute_log.jsonl"

    if files_path.exists() or cols_path.exists():
        print(
            f"error: existing {files_path.name} / {cols_path.name} — "
            f"remove or rename them before re-running (no resume yet).",
            file=sys.stderr,
        )
        return 2

    files_schema = pa.schema([
        ("file_path", pa.string()),
        ("file_content_sha256", pa.string()),
        ("n_cols", pa.int32()),
        ("non_trivial_cols", pa.int32()),
        ("total_rows", pa.int64()),
        ("rejects_non_trivial", pa.int64()),
        ("elapsed_s", pa.float64()),
        ("error", pa.string()),
    ])
    cols_schema = pa.schema([
        ("file_path", pa.string()),
        ("file_content_sha256", pa.string()),
        ("column_name", pa.string()),
        ("sense_prediction", pa.string()),
        ("sense_confidence", pa.float64()),
        ("ydf_prediction", pa.string()),
        ("ydf_confidence", pa.float64()),
        ("sample_values_truncated", pa.string()),
        ("is_trivial", pa.bool_()),
    ])

    paths_to_process = paths
    if args.max_files is not None:
        paths_to_process = paths[: args.max_files]
    n_paths = len(paths_to_process)
    print(f"execute: {n_paths} parquets, jobs={args.jobs}",
          file=sys.stderr)

    files_writer = pq.ParquetWriter(
        files_path, files_schema, compression="snappy",
    )
    cols_writer = pq.ParquetWriter(
        cols_path, cols_schema, compression="snappy",
    )
    log_fh = log_path.open("w")

    file_buffer: list[FileResult] = []
    col_buffer: list[ColumnResult] = []
    n_processed = 0
    n_measure_written = 0
    n_skipped_not_measure = 0
    n_errors = 0
    error_strings: dict[str, int] = {}
    t_start = time.perf_counter()

    def flush_file_buffer() -> None:
        nonlocal file_buffer, col_buffer
        if not file_buffer:
            return
        # Filter to only measure-half (those with non-empty SHA).
        keep = [f for f in file_buffer
                if f.file_content_sha256 or f.error]
        if keep:
            table = pa.table({
                "file_path": [f.file_path for f in keep],
                "file_content_sha256": [
                    f.file_content_sha256 for f in keep
                ],
                "n_cols": [f.n_cols for f in keep],
                "non_trivial_cols": [f.non_trivial_cols for f in keep],
                "total_rows": [f.total_rows for f in keep],
                "rejects_non_trivial": [
                    f.rejects_non_trivial for f in keep
                ],
                "elapsed_s": [f.elapsed_s for f in keep],
                "error": [f.error for f in keep],
            }, schema=files_schema)
            files_writer.write_table(table)
        if col_buffer:
            table = pa.table({
                "file_path": [c.file_path for c in col_buffer],
                "file_content_sha256": [
                    c.file_content_sha256 for c in col_buffer
                ],
                "column_name": [c.column_name for c in col_buffer],
                "sense_prediction": [c.sense_prediction for c in col_buffer],
                "sense_confidence": [c.sense_confidence for c in col_buffer],
                "ydf_prediction": [c.ydf_prediction for c in col_buffer],
                "ydf_confidence": [c.ydf_confidence for c in col_buffer],
                "sample_values_truncated": [
                    c.sample_values_truncated for c in col_buffer
                ],
                "is_trivial": [c.is_trivial for c in col_buffer],
            }, schema=cols_schema)
            cols_writer.write_table(table)
        file_buffer = []
        col_buffer = []

    with cf.ProcessPoolExecutor(
        max_workers=args.jobs,
        initializer=_worker_init,
        initargs=(args.finetype_bin, args.duckdb_bin, args.partition),
    ) as pool:
        for fr, cols in pool.map(
            _execute_one,
            (str(p) for p in paths_to_process),
            chunksize=8,
        ):
            n_processed += 1
            if fr.error:
                n_errors += 1
                # Keep error strings de-dup'd by first 80 chars
                key = (fr.error or "")[:80]
                error_strings[key] = error_strings.get(key, 0) + 1
            if fr.file_content_sha256 or fr.error:
                file_buffer.append(fr)
                col_buffer.extend(cols)
                if fr.file_content_sha256 and not fr.error:
                    n_measure_written += 1
                log_fh.write(json.dumps({
                    "path": fr.file_path,
                    "sha": fr.file_content_sha256 or None,
                    "n_cols": fr.n_cols,
                    "non_trivial_cols": fr.non_trivial_cols,
                    "total_rows": fr.total_rows,
                    "rejects_non_trivial": fr.rejects_non_trivial,
                    "elapsed_s": round(fr.elapsed_s, 4),
                    "error": fr.error,
                }) + "\n")
            else:
                n_skipped_not_measure += 1
            if len(file_buffer) >= args.batch_size:
                flush_file_buffer()
            if n_processed % 5000 == 0:
                elapsed = time.perf_counter() - t_start
                rate = n_processed / elapsed if elapsed else 0
                eta = (n_paths - n_processed) / rate if rate else 0
                print(
                    f"  {n_processed}/{n_paths} "
                    f"({n_measure_written} measure-half written, "
                    f"{n_errors} errors) "
                    f"@ {rate:.0f} files/s — ETA {eta/3600:.2f} h",
                    file=sys.stderr,
                )

    flush_file_buffer()
    files_writer.close()
    cols_writer.close()
    log_fh.close()

    elapsed = time.perf_counter() - t_start
    summary = {
        "n_paths_input": n_paths,
        "n_processed": n_processed,
        "n_measure_written": n_measure_written,
        "n_skipped_not_measure": n_skipped_not_measure,
        "n_errors": n_errors,
        "error_rate": round(n_errors / max(1, n_measure_written + n_errors), 6),
        "first_5_distinct_error_strings": [
            {"error": s, "count": n}
            for s, n in sorted(
                error_strings.items(), key=lambda x: -x[1]
            )[:5]
        ],
        "elapsed_seconds": round(elapsed, 1),
        "elapsed_hours": round(elapsed / 3600.0, 3),
        "files_parquet": str(files_path),
        "columns_parquet": str(cols_path),
    }
    (out_dir / "execute_summary.json").write_text(
        json.dumps(summary, indent=2) + "\n"
    )
    print(json.dumps(summary, indent=2))
    print(f"wrote {files_path}, {cols_path}", file=sys.stderr)
    return 0


def _run_fill_ydf(args) -> int:
    """Pass B — sequential YDF fill.

    Reads `eval/gittables/corpus_pass/columns.parquet` (produced by
    Pass A with ydf_prediction = NULL on every row). Loads YDF model
    once in the main process, splits each row's
    `sample_values_truncated` on U+2502 to recover the column's
    sample values, computes features (stats + char-bigram TF-IDF),
    predicts top-1 label + confidence, and writes a new
    `columns.parquet`. The old file is renamed to
    `columns.pre-ydf.parquet` as a checkpoint.

    Single-process by design — avoids the per-worker model load
    that crashed Pass A at jobs=16. YDF inference is fast (~10 ms
    per column with stats+bigram features), so ~6M columns should
    complete in ~8-12 hours single-core."""
    try:
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
        import ydf  # type: ignore
        import pandas as pd  # type: ignore
    except ImportError as exc:
        print(f"error: dependency missing ({exc}). Activate venv.",
              file=sys.stderr)
        return 2

    out_dir = args.out_dir / "corpus_pass"
    cols_path = out_dir / "columns.parquet"
    backup_path = out_dir / "columns.pre-ydf.parquet"
    new_path = out_dir / "columns.with-ydf.parquet"

    if not cols_path.exists():
        print(f"error: {cols_path} not found — run --execute first.",
              file=sys.stderr)
        return 2

    print(f"loading YDF model from {args.ydf_model}", file=sys.stderr)
    model = ydf.load_model(str(args.ydf_model))
    vocab = json.loads(args.ydf_vocab.read_text())
    label_classes = list(model.label_classes())
    print(f"  model loaded ({len(label_classes)} classes); "
          f"vocab {len(vocab)} bigrams", file=sys.stderr)

    print(f"reading {cols_path}", file=sys.stderr)
    table = pq.read_table(cols_path)
    n_rows = table.num_rows
    print(f"  {n_rows} column rows to fill", file=sys.stderr)

    sample_arr = table["sample_values_truncated"].to_pylist()
    batch_size = args.batch_size or 10_000
    ydf_pred: list[str | None] = [None] * n_rows
    ydf_conf: list[float | None] = [None] * n_rows
    n_filled = 0
    n_empty = 0
    t_start = time.perf_counter()

    for batch_start in range(0, n_rows, batch_size):
        batch_end = min(batch_start + batch_size, n_rows)
        feat_rows = []
        valid_idx: list[int] = []
        for i in range(batch_start, batch_end):
            raw = sample_arr[i] or ""
            values = [v for v in raw.split(SAMPLE_JOIN_SEP) if v]
            if not values:
                n_empty += 1
                continue
            feat_rows.append(_features(values, vocab))
            valid_idx.append(i)
        if not feat_rows:
            continue
        df = pd.DataFrame(feat_rows)
        preds = model.predict(df)
        for k, i in enumerate(valid_idx):
            probs = preds[k]
            top_idx = max(range(len(probs)), key=lambda j: probs[j])
            ydf_pred[i] = label_classes[top_idx]
            ydf_conf[i] = float(probs[top_idx])
            n_filled += 1

        if batch_end % (batch_size * 10) == 0 or batch_end == n_rows:
            elapsed = time.perf_counter() - t_start
            rate = batch_end / elapsed if elapsed else 0
            eta = (n_rows - batch_end) / rate if rate else 0
            print(
                f"  {batch_end}/{n_rows} "
                f"({n_filled} filled, {n_empty} empty) "
                f"@ {rate:.0f} rows/s — ETA {eta/3600:.2f} h",
                file=sys.stderr,
            )

    print("rewriting columns.parquet with YDF predictions...",
          file=sys.stderr)
    new_table = table.set_column(
        table.schema.get_field_index("ydf_prediction"),
        "ydf_prediction",
        pa.array(ydf_pred, type=pa.string()),
    ).set_column(
        table.schema.get_field_index("ydf_confidence"),
        "ydf_confidence",
        pa.array(ydf_conf, type=pa.float64()),
    )

    # JSON Schema validation gate on YDF predictions (per spec
    # 2026-05-26-ydf-validation-gate ac-06). Adds two columns:
    #   ydf_validation_pass_rate: fraction of values matching the
    #     predicted label's JSON Schema (1.0 when no applicable
    #     validation or no samples).
    #   ydf_prediction_gated: NULL when pass_rate < 0.5, else the
    #     original prediction. Downstream consumers prefer this
    #     column; the raw ydf_prediction stays for lens-disagreement
    #     diagnostics.
    print("applying YDF JSON Schema validation gate...", file=sys.stderr)
    try:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        from apply_ydf_validation_gate import (  # noqa: E402
            PASS_RATE_THRESHOLD, evaluate, load_validations,
        )
    except ImportError as exc:
        print(f"warn: validation gate import failed ({exc}); "
              "writing parquet WITHOUT gate columns.", file=sys.stderr)
        pq.write_table(new_table, new_path, compression="snappy")
    else:
        specs = load_validations()
        pass_rates: list[float | None] = [None] * n_rows
        gated: list[str | None] = [None] * n_rows
        n_refused = 0
        for i in range(n_rows):
            label = ydf_pred[i]
            if label is None:
                continue
            spec = specs.get(label)
            raw = sample_arr[i] or ""
            values = [v for v in raw.split(SAMPLE_JOIN_SEP) if v]
            rate = evaluate(spec, values)
            pass_rates[i] = rate
            if rate < PASS_RATE_THRESHOLD:
                n_refused += 1
            else:
                gated[i] = label
        new_table = new_table.append_column(
            "ydf_validation_pass_rate",
            pa.array(pass_rates, type=pa.float64()),
        ).append_column(
            "ydf_prediction_gated", pa.array(gated, type=pa.string()),
        )
        print(f"  gate refused {n_refused:,} of {n_filled:,} YDF "
              f"predictions ({n_refused / max(n_filled, 1) * 100:.2f}%)",
              file=sys.stderr)
        pq.write_table(new_table, new_path, compression="snappy")
    cols_path.rename(backup_path)
    new_path.rename(cols_path)

    elapsed = time.perf_counter() - t_start
    summary = {
        "n_rows": n_rows,
        "n_filled": n_filled,
        "n_empty_samples": n_empty,
        "elapsed_seconds": round(elapsed, 1),
        "elapsed_hours": round(elapsed / 3600.0, 3),
        "ydf_model": str(args.ydf_model),
        "columns_parquet": str(cols_path),
        "backup_pre_ydf_parquet": str(backup_path),
    }
    (out_dir / "fill_ydf_summary.json").write_text(
        json.dumps(summary, indent=2) + "\n"
    )
    print(json.dumps(summary, indent=2))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Gittables multi-lens corpus pass (ac-04 dry-run / ac-06 execute)"
    )
    parser.add_argument(
        "--corpus-index", type=Path, default=DEFAULT_CORPUS_INDEX,
    )
    parser.add_argument("--ydf-model", type=Path, default=DEFAULT_YDF_MODEL)
    parser.add_argument("--ydf-vocab", type=Path, default=DEFAULT_YDF_VOCAB)
    parser.add_argument("--finetype-bin", default=DEFAULT_FINETYPE)
    parser.add_argument("--duckdb-bin", default=DEFAULT_DUCKDB)
    parser.add_argument("--jobs", type=int, default=1)
    parser.add_argument("--max-files", type=int, default=None)
    parser.add_argument(
        "--partition", choices=("measure", "calibrate"), default="measure",
        help="Which SHA partition to process. Default 'measure' (SHA %% 2 == 1) "
             "is ac-06's full corpus pass. 'calibrate' (SHA %% 2 == 0) is "
             "reserved for non-measurement traffic; ac-13's reproducibility "
             "sub-pass runs against the calibrate half so debugging iterations "
             "don't contaminate the measure half.",
    )
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument(
        "--dry-run", action="store_true",
        help="Sample measure-half files, time each stage, project to "
             "full corpus. No Parquet outputs.",
    )
    mode.add_argument(
        "--execute", action="store_true",
        help="ac-06 Pass A — profile + validate at high parallelism. "
             "Writes files.parquet + columns.parquet (ydf_prediction "
             "= NULL). Workers do NOT load the YDF model.",
    )
    mode.add_argument(
        "--fill-ydf", action="store_true",
        help="ac-06 Pass B — single-process sequential YDF fill. "
             "Reads columns.parquet, computes features + predictions, "
             "writes new columns.parquet (old kept as "
             "columns.pre-ydf.parquet).",
    )
    parser.add_argument(
        "--out-dir", type=Path, default=DEFAULT_OUT_DIR,
    )
    parser.add_argument(
        "--batch-size", type=int, default=500,
        help="Files per parquet row group during --execute (default 500).",
    )
    args = parser.parse_args()

    if not args.corpus_index.exists():
        print(f"error: corpus index not found: {args.corpus_index}",
              file=sys.stderr)
        return 2

    try:
        import ydf  # type: ignore
    except ImportError as exc:
        print(f"error: ydf import failed ({exc}). Activate venv: "
              "source eval/gittables/.venv/bin/activate",
              file=sys.stderr)
        return 2

    if args.fill_ydf:
        return _run_fill_ydf(args)

    paths: list[Path] = []
    with args.corpus_index.open() as fh:
        for line in fh:
            line = line.strip()
            if line and not line.startswith("#"):
                paths.append(Path(line))
    n_total = len(paths)
    print(f"corpus_paths.txt: {n_total} parquets", file=sys.stderr)

    if args.execute:
        return _run_execute(args, paths)

    # --- dry-run branch ---
    ydf_model = ydf.load_model(str(args.ydf_model))
    ydf_vocab = json.loads(args.ydf_vocab.read_text())

    target = args.max_files or 1000
    print(f"dry-run: collecting {target} MEASURE-half samples "
          f"(scanning sequentially from corpus_paths)", file=sys.stderr)

    timings: list[FileTimings] = []
    scanned = 0
    with cf.ProcessPoolExecutor(max_workers=args.jobs) as pool:
        futures = []
        for p in paths:
            futures.append(pool.submit(
                _measure_one, p,
                finetype_bin=args.finetype_bin,
                duckdb_bin=args.duckdb_bin,
                ydf_model=None,  # avoid pickling — see note below
                ydf_vocab=ydf_vocab,
                measure_only=True,
            ))
            scanned += 1
            if scanned >= target * 4:
                break
        for fut in cf.as_completed(futures):
            r = fut.result()
            if r.in_measure_half and r.error is None:
                timings.append(r)
                if len(timings) >= target:
                    for f in futures:
                        f.cancel()
                    break

    n_measure_sampled = len(timings)
    print(
        f"  measure-half samples collected: {n_measure_sampled} / target {target} "
        f"(scanned {scanned})",
        file=sys.stderr,
    )

    if not timings:
        print("error: no measure-half samples collected", file=sys.stderr)
        return 2

    # NOTE: pool can't pickle ydf model; re-time ydf separately in-process.
    print("re-timing YDF inference in-process (sequential)...", file=sys.stderr)
    ydf_times: list[float] = []
    for r in timings[:min(50, len(timings))]:
        # Reconstruct the per-file pipeline just for YDF timing.
        try:
            with tempfile.TemporaryDirectory(prefix="ftcp-ydf-") as td:
                tmp = Path(td)
                csv_path = tmp / "in.csv"
                _parquet_to_csv(Path(r.path), csv_path, args.duckdb_bin)
                t = time.perf_counter()
                cs = _read_column_samples(csv_path, args.duckdb_bin)
                _ = _ydf_predict_columns(cs, ydf_model, ydf_vocab)
                ydf_times.append(time.perf_counter() - t)
        except Exception:  # noqa: BLE001
            continue
    ydf_mean_s = sum(ydf_times) / len(ydf_times) if ydf_times else 0.0

    n_sampled = len(timings)
    mean_sha256 = sum(r.sha256_s for r in timings) / n_sampled
    mean_p2c = sum(r.parquet_to_csv_s for r in timings) / n_sampled
    mean_profile = sum(r.profile_s for r in timings) / n_sampled
    mean_validate = sum(r.validate_s for r in timings) / n_sampled
    mean_dbpedia = sum(r.dbpedia_kv_s for r in timings) / n_sampled
    baseline_pv = mean_p2c + mean_profile + mean_validate
    mean_total_per_file = (
        mean_sha256 + baseline_pv + ydf_mean_s + mean_dbpedia
    )
    measure_half_files_estimate = n_total // 2
    projected_total_s_single = (
        measure_half_files_estimate * mean_total_per_file
    )
    projected_total_s_jobs = projected_total_s_single / max(1, args.jobs)
    projected_h_jobs = projected_total_s_jobs / 3600.0

    out = {
        "n_corpus_total": n_total,
        "n_measure_half_estimate": measure_half_files_estimate,
        "n_sampled": n_sampled,
        "jobs": args.jobs,
        "per_file_means_s": {
            "sha256": round(mean_sha256, 4),
            "baseline_profile_validate_s": round(baseline_pv, 4),
            "ydf_inference_s": round(ydf_mean_s, 4),
            "dbpedia_kv_extraction_s": round(mean_dbpedia, 4),
            "total_s": round(mean_total_per_file, 4),
        },
        "projected_full_corpus": {
            "wall_clock_seconds_single_core": round(projected_total_s_single, 1),
            "wall_clock_seconds_at_jobs": round(projected_total_s_jobs, 1),
            "wall_clock_hours_at_jobs": round(projected_h_jobs, 2),
            "target_hours": 48.0,
            "ceiling_hours": 72.0,
            "soft_fail": projected_h_jobs > 48.0 and projected_h_jobs <= 72.0,
            "amend_required": projected_h_jobs > 72.0,
        },
    }
    args.out_dir.mkdir(parents=True, exist_ok=True)
    out_path = args.out_dir / "corpus_paths_dryrun.json"
    out_path.write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))
    print(f"wrote {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
