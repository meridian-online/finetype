#!/usr/bin/env python3
"""Per-cycle working-slice driver — autonomy contract §2 step 2-6.

Implements the cron-firing agent's per-cycle work loop:
  1. Sample N=3000 fresh files from the working slice (round-robin
     invariant; tracked in coverage_log).
  2. For each: parquet→csv (DuckDB), profile, validate (lenient).
  3. Classify outcome per §3 branches (B01/B03/B04/B05/B06).
  4. Append to working_slice_coverage.tsv (always), failure_log.tsv
     (B01/B04 misclassification), harvest_pool.tsv (B05 generator
     widening candidate).

Exclusion rules per contract §2 invariants:
  - holdout files (eval/gittables/holdout_paths.txt) — excluded
  - harvest_pool files (eval/gittables/harvest_pool.tsv) — excluded
  - already-visited this sprint — deprioritised (round-robin)

Round-robin invariant: every working-slice file visited at least once
before any file is visited twice. Implemented as: prefer min-visit-count
content_hashes when sampling.

Emits a JSON summary on stdout for the cycle log assembler:
  {
    "files_sampled": 3000,
    "files_processed": 2987,
    "branches_taken": ["B03", "B03", "B04", "B06", "B01", ...],
    "outcome_counts": {"clean_pass": ..., "classifier_quality_issue": ...,
                       "corpus_quality_issue": ...},
    "harvest_count": 12,
    "elapsed_s": 940.2
  }
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import hashlib
import json
import random
import sys
import time
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from append_log import (  # noqa: E402
    append_tsv_row,
    FAILURE_LOG_HEADER,
    COVERAGE_LOG_HEADER,
    HARVEST_POOL_HEADER,
)
from gittables_gate import (  # noqa: E402
    _column_types,
    _parquet_to_csv,
    _profile,
    _sql_str,
    _validate,
    TRIVIAL_TYPES,
)

DEFAULT_HOLDOUT = REPO_ROOT / "eval" / "gittables" / "holdout_paths.txt"
DEFAULT_COVERAGE_LOG = REPO_ROOT / "eval" / "gittables" / "working_slice_coverage.tsv"
DEFAULT_FAILURE_LOG = REPO_ROOT / "eval" / "gittables" / "failure_log.tsv"
DEFAULT_HARVEST_POOL = REPO_ROOT / "eval" / "gittables" / "harvest_pool.tsv"
DEFAULT_INFERENCE_SIGNALS = REPO_ROOT / "eval" / "gittables" / "inference_signals.tsv"
DEFAULT_CONTENT_HASH_CACHE = (
    REPO_ROOT / "eval" / "gittables" / ".content_hash_cache.tsv"
)
DEFAULT_FINETYPE = "finetype"
DEFAULT_DUCKDB = "duckdb"

# Per spec finetype-7zi ac-12: sidecar `inference_signals.tsv` 12-col schema.
INFERENCE_SIGNALS_HEADER: tuple[str, ...] = (
    "cycle_id",
    "timestamp",
    "file_path",
    "file_content_sha256",
    "column_name",
    "predicted_type",
    "inferred_correct_type",
    "confidence",
    "mechanism",
    "validator_pass_rate",
    "header_match",
    "top_k_json",
)

# Per contract §1: a column is "trivial" iff predicted as plain_text or
# fallback decimal_number. Reused from gittables_gate.TRIVIAL_TYPES.
TRIVIAL_FRACTION_FLOOR = 0.20  # B04 threshold
NON_TRIVIAL_FRACTION_FLOOR = 0.80  # B03 clean_pass requires same as gate

OBSERVED_SAMPLE_LIMIT = 8       # values per failure_log row
OBSERVED_SAMPLE_CHARS = 200      # max char budget for the joined sample
HARVEST_SAMPLE_LIMIT = 64        # values per harvest_pool row


@dataclass
class WorkOutcome:
    file_path: str
    file_content_sha256: str
    outcome: str   # clean_pass | classifier_quality_issue | corpus_quality_issue
    branches: list[str] = field(default_factory=list)
    type_distribution: dict[str, int] = field(default_factory=dict)
    failure_rows: list[dict] = field(default_factory=list)  # for failure_log
    harvest_rows: list[dict] = field(default_factory=list)  # for harvest_pool
    signal_rows: list[dict] = field(default_factory=list)   # for inference_signals (sidecar)
    elapsed_s: float = 0.0
    error: str | None = None


def _infer_and_build_failure_row(
    *,
    column_name: str,
    predicted_type: str,
    observed_values: list[str],
    harvest_samples: list[str],
    finetype_bin: str,
) -> tuple[dict, dict, dict]:
    """Run `finetype infer --mode column --batch --explain` over
    (column_name, predicted_type, samples). Historically `finetype
    infer-type`; the verb was collapsed into a flag on `infer`. Wire
    shape unchanged.

    Returns three dicts:
      - failure_row: 5 fields written to failure_log.tsv (joined with cycle/file
        headers in `_write_logs`). The 9-col schema is preserved; this helper
        fills `inferred_correct_type` and `mechanism` from real inference
        output instead of the deprecated hardcoded literals.
      - harvest_row: 3 fields for harvest_pool.tsv (target_generator copies
        the predicted type for B01; `(trivial-fallback)` for B04 — caller
        overrides via the returned dict).
      - signal_row: 8 fields for the sidecar inference_signals.tsv (joined
        with cycle/file headers in `_write_logs`). Confidence + signal
        sub-scores live here, not in failure_log.tsv (ac-12).

    Per spec finetype-7zi:
      - ac-15: samples truncated to N=8 inside the binary.
      - ac-09: mechanism is from the closed 10-token vocabulary.
      - ac-04: output is deterministic (byte-identical re-run).

    Subprocess errors fall back to the same `unknown` / `fallthrough` /
    confidence=0.0 emission as the empty-samples precondition (spec ac-10).
    """
    import subprocess
    payload = json.dumps(
        {
            "column_name": column_name,
            "predicted_type": predicted_type,
            "samples": observed_values,
        }
    )
    inferred_correct_type = "unknown"
    confidence = 0.0
    mechanism = "fallthrough"
    validator_pass_rate = 0.0
    header_match = 0.0
    top_k_json = "[]"
    try:
        res = subprocess.run(
            [finetype_bin, "infer", "--mode", "column", "--batch", "--explain"],
            input=payload,
            capture_output=True,
            text=True,
            timeout=10,
        )
        if res.returncode == 0:
            out = json.loads(res.stdout.strip())
            inferred_correct_type = out.get("inferred_correct_type", "unknown")
            confidence = float(out.get("confidence", 0.0))
            mechanism = out.get("mechanism", "fallthrough")
            signals = out.get("signals", {}) or {}
            validator_pass_rate = float(signals.get("validator_pass_rate", 0.0))
            header_match = float(signals.get("header_match", 0.0))
            top_k_json = json.dumps(signals.get("top_k", []), sort_keys=True)
    except Exception:
        # Subprocess crash / timeout / parse error: fall through to defaults.
        pass

    failure_row = {
        "column_name": column_name,
        "predicted_type": predicted_type,
        "observed_values_sample": _truncate_sample(observed_values),
        "inferred_correct_type": inferred_correct_type,
        "mechanism": mechanism,
    }
    harvest_row = {
        "column_name": column_name,
        "target_generator": predicted_type,
        "samples_json": json.dumps(harvest_samples),
    }
    signal_row = {
        "column_name": column_name,
        "predicted_type": predicted_type,
        "inferred_correct_type": inferred_correct_type,
        "confidence": confidence,
        "mechanism": mechanism,
        "validator_pass_rate": validator_pass_rate,
        "header_match": header_match,
        "top_k_json": top_k_json,
    }
    return failure_row, harvest_row, signal_row


def _read_path_set(path: Path) -> set[str]:
    """Read a paths file (one path per line, # comments skipped)."""
    out: set[str] = set()
    if not path.exists():
        return out
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            out.add(line)
    return out


def _read_content_hash_cache(path: Path) -> list[tuple[str, str]]:
    """Return list of (file_path, content_sha256). Skips header + comments."""
    rows: list[tuple[str, str]] = []
    if not path.exists():
        raise FileNotFoundError(
            f"content-hash cache missing at {path} — run gittables_holdout_freeze.py first"
        )
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) < 4:
                continue
            rows.append((parts[0], parts[3]))
    return rows


def _read_visit_counts(coverage_log: Path) -> Counter:
    """Return Counter[content_sha256] of how many times each file has been visited."""
    counts: Counter = Counter()
    if not coverage_log.exists():
        return counts
    with open(coverage_log, encoding="utf-8") as fh:
        next(fh, None)  # header
        for line in fh:
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 4:
                counts[parts[3]] += 1  # file_content_sha256 column
    return counts


def _read_harvest_excludes(harvest_pool: Path) -> set[str]:
    """Return set of content_sha256 values permanently excluded by harvest."""
    out: set[str] = set()
    if not harvest_pool.exists():
        return out
    with open(harvest_pool, encoding="utf-8") as fh:
        next(fh, None)  # header
        for line in fh:
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 4:
                out.add(parts[3])
    return out


def _holdout_content_hashes(
    holdout_paths: set[str], cache: list[tuple[str, str]]
) -> set[str]:
    by_path = {p: s for p, s in cache}
    return {by_path[p] for p in holdout_paths if p in by_path}


def _sample_working_slice(
    cache: list[tuple[str, str]],
    *,
    holdout_hashes: set[str],
    harvest_hashes: set[str],
    visit_counts: Counter,
    n: int,
    seed: int,
) -> list[tuple[str, str]]:
    """Round-robin sample N (path, content_sha256) tuples.

    Strategy: bucket files by visit count; sample from min-count buckets
    first. Within a bucket, deterministic random shuffle by seed.
    Each file appears at most once in the cache (path-keyed); content
    duplicates across topic dirs are NOT collapsed at this layer (the
    coverage log records by content_hash so they collapse in counts).
    """
    rng = random.Random(seed)
    eligible: list[tuple[str, str]] = [
        (p, s) for p, s in cache
        if s not in holdout_hashes and s not in harvest_hashes
    ]
    # Group by visit count
    by_count: dict[int, list[tuple[str, str]]] = defaultdict(list)
    for entry in eligible:
        by_count[visit_counts[entry[1]]].append(entry)
    out: list[tuple[str, str]] = []
    for visit_n in sorted(by_count.keys()):
        bucket = by_count[visit_n]
        rng.shuffle(bucket)
        take = min(len(bucket), n - len(out))
        out.extend(bucket[:take])
        if len(out) >= n:
            break
    return out


def _truncate_sample(values: list[str]) -> str:
    """Pack up to OBSERVED_SAMPLE_LIMIT values into a │-separated string capped at OBSERVED_SAMPLE_CHARS."""
    out: list[str] = []
    used = 0
    for v in values[:OBSERVED_SAMPLE_LIMIT]:
        s = str(v)
        # Strip newlines/tabs from individual values
        s = s.replace("\t", " ").replace("\n", " ").replace("\r", " ")
        # +1 for separator
        if used + len(s) + 1 > OBSERVED_SAMPLE_CHARS:
            break
        out.append(s)
        used += len(s) + 1
    return "│".join(out)


def _column_rejects(
    db_path: Path, duckdb_bin: str, columns: list[str]
) -> dict[str, int]:
    """Return {column_name: distinct_row_reject_count} from finetype_reject_errors.

    Empty dict on any error or empty column list.
    """
    if not columns:
        return {}
    in_list = ",".join(_sql_str(c) for c in sorted(set(columns)))
    sql = (
        "SELECT column_name, COUNT(DISTINCT line) "
        "FROM finetype_reject_errors "
        f"WHERE column_name IN ({in_list}) "
        "GROUP BY column_name;"
    )
    import subprocess
    res = subprocess.run(
        [duckdb_bin, "-noheader", "-csv", str(db_path), sql],
        capture_output=True, text=True, timeout=30,
    )
    if res.returncode != 0:
        return {}
    out: dict[str, int] = {}
    for line in res.stdout.strip().splitlines():
        parts = line.split(",", 1)
        if len(parts) == 2:
            name = parts[0].strip().strip('"')
            try:
                out[name] = int(parts[1].strip())
            except ValueError:
                continue
    return out


def _column_value_samples(
    csv_path: Path, columns: list[str], n: int
) -> dict[str, list[str]]:
    """Read first `n` non-empty values per requested column from the CSV."""
    if not columns:
        return {}
    import csv as _csv
    columns_set = set(columns)
    samples: dict[str, list[str]] = {c: [] for c in columns_set}
    try:
        with open(csv_path, encoding="utf-8", errors="replace") as fh:
            reader = _csv.DictReader(fh)
            row_count = 0
            for row in reader:
                row_count += 1
                if row_count > 1000:  # cap scan
                    break
                done = True
                for c in columns_set:
                    if len(samples[c]) >= n:
                        continue
                    done = False
                    v = row.get(c)
                    if v is None or v == "":
                        continue
                    samples[c].append(v)
                if done:
                    break
    except Exception:
        pass
    return samples


def _classify_and_log(
    parquet: Path,
    content_sha: str,
    *,
    finetype_bin: str,
    duckdb_bin: str,
) -> WorkOutcome:
    import tempfile
    t0 = time.perf_counter()
    out = WorkOutcome(
        file_path=str(parquet),
        file_content_sha256=content_sha,
        outcome="corpus_quality_issue",
    )
    with tempfile.TemporaryDirectory(prefix="finetype-cycle-") as td:
        tmp = Path(td)
        csv_path = tmp / "in.csv"
        schema_path = tmp / "in.schema.json"
        db_path = tmp / "in.db"
        try:
            _parquet_to_csv(parquet, csv_path, duckdb_bin)
        except Exception as exc:
            out.outcome = "corpus_quality_issue"
            out.branches.append("B06")
            out.error = f"parquet_read: {type(exc).__name__}: {exc}"[:400]
            out.elapsed_s = round(time.perf_counter() - t0, 3)
            return out

        try:
            _profile(csv_path, schema_path, finetype_bin)
        except Exception as exc:
            out.outcome = "corpus_quality_issue"
            out.branches.append("B06")
            out.error = f"profile: {type(exc).__name__}: {exc}"[:400]
            out.elapsed_s = round(time.perf_counter() - t0, 3)
            return out

        try:
            summary = _validate(csv_path, schema_path, db_path, finetype_bin)
        except Exception as exc:
            out.outcome = "corpus_quality_issue"
            out.branches.append("B06")
            out.error = f"validate: {type(exc).__name__}: {exc}"[:400]
            out.elapsed_s = round(time.perf_counter() - t0, 3)
            return out

        col_types = _column_types(schema_path)
        n_cols = len(col_types)
        if n_cols == 0:
            out.outcome = "corpus_quality_issue"
            out.branches.append("B06")
            out.error = "no columns in profile schema"
            out.elapsed_s = round(time.perf_counter() - t0, 3)
            return out

        # Type distribution
        out.type_distribution = dict(Counter(col_types.values()))

        non_trivial_cols = {
            c for c, t in col_types.items() if t and t not in TRIVIAL_TYPES
        }
        trivial_cols = {
            c for c, t in col_types.items() if not t or t in TRIVIAL_TYPES
        }
        non_trivial_pct = len(non_trivial_cols) / n_cols
        trivial_pct = len(trivial_cols) / n_cols

        total_rows = int(summary.get("total_rows", 0))

        # Per-column reject counts on non-trivial cols (for B01 detection)
        reject_counts = _column_rejects(db_path, duckdb_bin, sorted(non_trivial_cols))
        total_rejects_nt = sum(reject_counts.values())

        # B01: any non-trivial col with reject_count == row_count
        b01_columns: list[str] = [
            c for c, n_rej in reject_counts.items()
            if total_rows > 0 and n_rej >= total_rows
        ]

        if b01_columns:
            # Misclassification path
            out.branches.append("B01")
            out.outcome = "classifier_quality_issue"
            samples = _column_value_samples(
                csv_path, b01_columns, OBSERVED_SAMPLE_LIMIT
            )
            harvest_samples = _column_value_samples(
                csv_path, b01_columns, HARVEST_SAMPLE_LIMIT
            )
            for c in b01_columns:
                obs = samples.get(c, [])
                fr, hr, sr = _infer_and_build_failure_row(
                    column_name=c,
                    predicted_type=col_types.get(c, ""),
                    observed_values=obs,
                    harvest_samples=harvest_samples.get(c, []),
                    finetype_bin=finetype_bin,
                )
                out.failure_rows.append(fr)
                out.signal_rows.append(sr)
                # B05: harvest these
                out.harvest_rows.append(hr)
                out.branches.append("B05")
        elif total_rejects_nt == 0 and non_trivial_pct >= NON_TRIVIAL_FRACTION_FLOOR:
            # B03 clean pass
            out.outcome = "clean_pass"
            out.branches.append("B03")
        elif total_rejects_nt == 0 and trivial_pct >= TRIVIAL_FRACTION_FLOOR:
            # B04 trivial-fallback
            out.outcome = "classifier_quality_issue"
            out.branches.append("B04")
            harvest_samples = _column_value_samples(
                csv_path, sorted(trivial_cols), HARVEST_SAMPLE_LIMIT
            )
            for c in sorted(trivial_cols):
                fr, hr, sr = _infer_and_build_failure_row(
                    column_name=c,
                    predicted_type=col_types.get(c, ""),
                    observed_values=harvest_samples.get(c, []),
                    harvest_samples=harvest_samples.get(c, []),
                    finetype_bin=finetype_bin,
                )
                # B04's harvest target_generator is `(trivial-fallback)`,
                # not the predicted type. Override the helper's default.
                hr["target_generator"] = "(trivial-fallback)"
                out.failure_rows.append(fr)
                out.signal_rows.append(sr)
                out.harvest_rows.append(hr)
                out.branches.append("B05")
        else:
            # Mixed — partial rejects on non-trivial; could be B02 (validator
            # narrow), but B02 requires a sibling-pattern audit which cannot
            # be done autonomously per MADR 0078. Log as classifier_quality
            # for visibility; do NOT add failure_log entry (avoids B01 noise).
            out.outcome = "classifier_quality_issue"
            out.branches.append("B02-candidate")

    out.elapsed_s = round(time.perf_counter() - t0, 3)
    return out


def _process_one(
    args: tuple[str, str, str, str],
) -> WorkOutcome:
    """Worker entry — must accept a tuple for ProcessPoolExecutor."""
    parquet_str, content_sha, finetype_bin, duckdb_bin = args
    return _classify_and_log(
        Path(parquet_str), content_sha,
        finetype_bin=finetype_bin, duckdb_bin=duckdb_bin,
    )


def _write_logs(
    outcome: WorkOutcome,
    *,
    cycle_id: str,
    timestamp_iso: str,
    coverage_log: Path,
    failure_log: Path,
    harvest_pool: Path,
    inference_signals: Path,
) -> None:
    # Coverage row always written.
    append_tsv_row(
        coverage_log,
        {
            "cycle_id": cycle_id,
            "timestamp": timestamp_iso,
            "file_path": outcome.file_path,
            "file_content_sha256": outcome.file_content_sha256,
            "outcome": outcome.outcome,
            "predicted_type_distribution_json": json.dumps(
                outcome.type_distribution, sort_keys=True
            ),
        },
        header=COVERAGE_LOG_HEADER,
    )
    # Failure rows
    for fr in outcome.failure_rows:
        append_tsv_row(
            failure_log,
            {
                "cycle_id": cycle_id,
                "timestamp": timestamp_iso,
                "file_path": outcome.file_path,
                "file_content_sha256": outcome.file_content_sha256,
                **fr,
            },
            header=FAILURE_LOG_HEADER,
        )
    # Inference signals — sidecar (ac-12). Append-only from creation; one
    # row per failure row (paired by (cycle_id, file_path,
    # file_content_sha256, column_name) per ac-12 join key).
    for sr in outcome.signal_rows:
        append_tsv_row(
            inference_signals,
            {
                "cycle_id": cycle_id,
                "timestamp": timestamp_iso,
                "file_path": outcome.file_path,
                "file_content_sha256": outcome.file_content_sha256,
                **sr,
            },
            header=INFERENCE_SIGNALS_HEADER,
        )
    # Harvest rows
    for hr in outcome.harvest_rows:
        append_tsv_row(
            harvest_pool,
            {
                "cycle_id": cycle_id,
                "timestamp": timestamp_iso,
                "file_path": outcome.file_path,
                "file_content_sha256": outcome.file_content_sha256,
                **hr,
            },
            header=HARVEST_POOL_HEADER,
        )


def main() -> int:
    parser = argparse.ArgumentParser(description="Per-cycle working-slice driver")
    parser.add_argument("--cycle-id", required=True, help="UUID from preamble")
    parser.add_argument("--max-files", type=int, default=3000)
    parser.add_argument("--seed", type=int, default=None,
                        help="Sampling seed (defaults to hash of cycle_id)")
    parser.add_argument("--jobs", type=int, default=1)
    parser.add_argument("--holdout", type=Path, default=DEFAULT_HOLDOUT)
    parser.add_argument("--coverage-log", type=Path, default=DEFAULT_COVERAGE_LOG)
    parser.add_argument("--failure-log", type=Path, default=DEFAULT_FAILURE_LOG)
    parser.add_argument("--harvest-pool", type=Path, default=DEFAULT_HARVEST_POOL)
    parser.add_argument(
        "--inference-signals",
        type=Path,
        default=DEFAULT_INFERENCE_SIGNALS,
        help="Sidecar TSV for per-row confidence + signal sub-scores (ac-12)",
    )
    parser.add_argument(
        "--content-hash-cache", type=Path, default=DEFAULT_CONTENT_HASH_CACHE
    )
    parser.add_argument("--finetype-bin", default=DEFAULT_FINETYPE)
    parser.add_argument("--duckdb-bin", default=DEFAULT_DUCKDB)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    seed = args.seed if args.seed is not None else (
        int(hashlib.sha256(args.cycle_id.encode()).hexdigest(), 16) % (2**31)
    )

    if not args.quiet:
        print(f"working-slice: cycle={args.cycle_id} seed={seed} N={args.max_files} jobs={args.jobs}",
              file=sys.stderr)

    # Build sample set
    cache = _read_content_hash_cache(args.content_hash_cache)
    holdout_paths = _read_path_set(args.holdout)
    holdout_hashes = _holdout_content_hashes(holdout_paths, cache)
    harvest_hashes = _read_harvest_excludes(args.harvest_pool)
    visit_counts = _read_visit_counts(args.coverage_log)

    sampled = _sample_working_slice(
        cache,
        holdout_hashes=holdout_hashes,
        harvest_hashes=harvest_hashes,
        visit_counts=visit_counts,
        n=args.max_files,
        seed=seed,
    )
    if not args.quiet:
        print(f"  cache={len(cache)} holdout={len(holdout_hashes)} harvest={len(harvest_hashes)} prior_visits={sum(visit_counts.values())}",
              file=sys.stderr)
        print(f"  sampled={len(sampled)} files for processing", file=sys.stderr)

    if not sampled:
        print(json.dumps({"files_sampled": 0, "files_processed": 0,
                          "branches_taken": [], "outcome_counts": {},
                          "harvest_count": 0, "elapsed_s": 0.0}))
        return 0

    t_start = time.perf_counter()
    outcomes: list[WorkOutcome] = []
    branches: list[str] = []
    outcome_counts: Counter = Counter()
    harvest_count = 0

    work = [(p, s, args.finetype_bin, args.duckdb_bin) for p, s in sampled]

    if args.jobs <= 1:
        for i, w in enumerate(work, 1):
            outcome = _process_one(w)
            outcomes.append(outcome)
            ts = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            _write_logs(
                outcome,
                cycle_id=args.cycle_id, timestamp_iso=ts,
                coverage_log=args.coverage_log,
                failure_log=args.failure_log,
                harvest_pool=args.harvest_pool,
                inference_signals=args.inference_signals,
            )
            branches.extend(outcome.branches)
            outcome_counts[outcome.outcome] += 1
            harvest_count += len(outcome.harvest_rows)
            if not args.quiet and (i % 100 == 0 or i == len(work)):
                rate = i / (time.perf_counter() - t_start)
                print(f"  {i}/{len(work)} ({outcome_counts['clean_pass']} clean, "
                      f"{outcome_counts['classifier_quality_issue']} cqi, "
                      f"{outcome_counts['corpus_quality_issue']} cqs, "
                      f"{rate:.1f} files/s)", file=sys.stderr)
    else:
        with cf.ProcessPoolExecutor(max_workers=args.jobs) as pool:
            futures = [pool.submit(_process_one, w) for w in work]
            for i, fut in enumerate(cf.as_completed(futures), 1):
                outcome = fut.result()
                outcomes.append(outcome)
                ts = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
                _write_logs(
                    outcome,
                    cycle_id=args.cycle_id, timestamp_iso=ts,
                    coverage_log=args.coverage_log,
                    failure_log=args.failure_log,
                    harvest_pool=args.harvest_pool,
                    inference_signals=args.inference_signals,
                )
                branches.extend(outcome.branches)
                outcome_counts[outcome.outcome] += 1
                harvest_count += len(outcome.harvest_rows)
                if not args.quiet and (i % 100 == 0 or i == len(work)):
                    rate = i / (time.perf_counter() - t_start)
                    print(f"  {i}/{len(work)} ({outcome_counts['clean_pass']} clean, "
                          f"{outcome_counts['classifier_quality_issue']} cqi, "
                          f"{outcome_counts['corpus_quality_issue']} cqs, "
                          f"{rate:.1f} files/s)", file=sys.stderr)

    elapsed = round(time.perf_counter() - t_start, 3)
    summary = {
        "files_sampled": len(sampled),
        "files_processed": len(outcomes),
        "branches_taken": branches,
        "outcome_counts": dict(outcome_counts),
        "harvest_count": harvest_count,
        "elapsed_s": elapsed,
    }
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
