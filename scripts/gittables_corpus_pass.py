#!/usr/bin/env python3
"""Full-corpus pass for the gittables multi-lens diagnostic.

Spec: `.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/spec.yaml`
ac-04 (corpus index + dry-run runtime budget); ac-06 (full execute).

Extends `scripts/gittables_gate.py`'s per-file profile + validate
pipeline with two new stages — YDF lens inference and DBpedia /
Schema.org KV-metadata extraction. Runs on the MEASURE half of the
corpus (`file_content_sha256 MOD 2 == 1`), enumerated from a committed
`eval/gittables/corpus_paths.txt`.

Modes:
  --dry-run   Times the per-file pipeline on a sample (--max-files
              measure-half hits), projects linearly to full corpus,
              emits `eval/gittables/corpus_paths_dryrun.json`.
              No Parquet outputs.
  (default)   Full execute — writes
              eval/gittables/corpus_pass/{files,columns}.parquet etc.
              (ac-06; not yet wired in this commit).

Usage (from a venv with ydf + duckdb + pandas):
  source eval/gittables/.venv/bin/activate
  python3 scripts/gittables_corpus_pass.py --jobs 16 --max-files 1000 --dry-run
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
        "--dry-run", action="store_true",
        help="Sample measure-half files, time each stage, project to "
             "full corpus. No Parquet outputs.",
    )
    parser.add_argument(
        "--out-dir", type=Path, default=DEFAULT_OUT_DIR,
    )
    args = parser.parse_args()

    if not args.dry_run:
        print(
            "error: full --execute mode is ac-06 (not yet wired); "
            "pass --dry-run for ac-04.",
            file=sys.stderr,
        )
        return 2

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

    ydf_model = ydf.load_model(str(args.ydf_model))
    ydf_vocab = json.loads(args.ydf_vocab.read_text())

    paths: list[Path] = []
    with args.corpus_index.open() as fh:
        for line in fh:
            line = line.strip()
            if line and not line.startswith("#"):
                paths.append(Path(line))
    n_total = len(paths)
    print(f"corpus_paths.txt: {n_total} parquets", file=sys.stderr)

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
