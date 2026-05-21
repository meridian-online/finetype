#!/usr/bin/env python3
"""GitTables holdout round-trip gate measurement.

Implements the autonomy contract gate metric (§1) — bead finetype-e6d.

For each file F in eval/gittables/holdout_paths.txt:
  1. Convert parquet → CSV via DuckDB COPY (profile is CSV-only).
  2. finetype profile -f F.csv -o json-schema > F.schema.json
  3. finetype validate F.csv F.schema.json --db F.db --table T --lenient
  4. Read reject sidecar (finetype_reject_errors) for per-column counts.
  5. Apply pass criterion:
       (a) ≥ 80% of columns predicted as non-trivial type, AND
       (b) ≤ 1% of rows rejected on the non-trivial columns
     Trivial types:
       - representation.text.plain_text
       - representation.numeric.decimal_number

Emits a machine-readable JSON summary on stdout (per e6d ac-02):

    {
      "gate_score": 0.844,
      "files_passed": 1688,
      "files_total": 2000,
      "model_sha": "<sha256 of models/default/model.safetensors>",
      "model_tag": "<resolved-symlink-name>",
      "harness_sha": "<git rev of HEAD>",
      "harness_dirty": false,
      "elapsed_s": 3450.2,
      "per_file": [ {path, passed, n_cols, non_trivial_cols, rejects, ...} ]
    }

Failure modes:
  - Per-file errors (parquet read, profile crash, validate crash) are
    counted as fails and recorded with a "skipped: <reason>" note.
    They do NOT halt the cycle — that's branch B06's job in the contract.

Reproducibility (e6d ac-05):
  Re-running on identical input (same holdout_paths.txt + same model
  symlink + same finetype binary) produces the same gate_score modulo
  per-file errors that are themselves reproducible.

Performance (e6d ac-04):
  Target <60min on M1 for 2000 files. Single-process sequential is the
  v1; --jobs N flag enables ProcessPoolExecutor for parallelism.

Usage:
    python3 scripts/gittables_gate.py
    python3 scripts/gittables_gate.py --holdout eval/gittables/holdout_paths.txt
    python3 scripts/gittables_gate.py --jobs 4
    python3 scripts/gittables_gate.py --max-files 10  # smoke test
"""

from __future__ import annotations

import argparse
import concurrent.futures as cf
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, asdict
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_HOLDOUT = REPO_ROOT / "eval" / "gittables" / "holdout_paths.txt"
DEFAULT_MODEL_DIR = REPO_ROOT / "models" / "default"
DEFAULT_FINETYPE = "finetype"
DEFAULT_DUCKDB = "duckdb"

# Trivial type set per autonomy contract §1.
# A column is "non-trivial" iff its predicted x-finetype-label is NOT one
# of these. Anything else (currency, identifier.increment, locale-specific
# datetime, etc.) is what we want the model to be predicting.
TRIVIAL_TYPES: frozenset[str] = frozenset({
    "representation.text.plain_text",
    "representation.numeric.decimal_number",
})

# Pass criterion thresholds (e6d ac-02 / contract §1).
NON_TRIVIAL_FLOOR = 0.80   # at least 80% of columns must be non-trivial
REJECT_RATE_CEIL = 0.01    # at most 1% of rows can be rejected on non-trivial cols


@dataclass
class FileOutcome:
    path: str
    passed: bool
    n_cols: int = 0
    non_trivial_cols: int = 0
    non_trivial_pct: float = 0.0
    total_rows: int = 0
    rejects_non_trivial: int = 0
    reject_rate_non_trivial: float = 0.0
    elapsed_s: float = 0.0
    error: str | None = None


def _read_holdout(path: Path) -> list[Path]:
    out: list[Path] = []
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            out.append(Path(line))
    return out


def _git_head(repo: Path) -> tuple[str, bool]:
    """Returns (sha, dirty). dirty=True means uncommitted changes present."""
    try:
        sha = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "HEAD"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
        status = subprocess.run(
            ["git", "-C", str(repo), "status", "--porcelain"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
        return sha, bool(status)
    except (subprocess.CalledProcessError, FileNotFoundError):
        return "(unknown)", False


def _model_identity(model_dir: Path) -> tuple[str, str, str]:
    """Returns (tag, weights_filename, sha256_hex). '(unavailable)' on error."""
    if not model_dir.exists():
        return ("(unavailable)", "(unavailable)", "(unavailable)")
    resolved = model_dir.resolve()
    tag = resolved.name
    candidate = resolved / "model.safetensors"
    if not candidate.exists():
        for ext in (".safetensors", ".bin"):
            for child in sorted(resolved.iterdir()):
                if child.suffix == ext:
                    candidate = child
                    break
            if candidate.exists():
                break
    if not candidate.exists():
        return (tag, "(unavailable)", "(unavailable)")
    h = hashlib.sha256()
    with open(candidate, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return (tag, candidate.name, h.hexdigest())


def _normalise_column_names(originals: list[str]) -> list[str]:
    """Strip whitespace from column names; dedupe collisions; rename empties.

    DuckDB's CSV reader strips leading/trailing whitespace from column names
    during binding, while pyarrow/parquet preserve them verbatim. Profile
    emits the verbatim name, validate references that name in SQL, DuckDB
    can't bind it → Binder Error. Normalising at the parquet→CSV boundary
    keeps profile and validate in agreement.
    """
    out: list[str] = []
    seen: dict[str, int] = {}
    for i, name in enumerate(originals):
        clean = name.strip()
        if not clean:
            clean = f"_col_{i}"
        if clean in seen:
            seen[clean] += 1
            clean = f"{clean}_{seen[clean]}"
        else:
            seen[clean] = 0
        out.append(clean)
    return out


def _parquet_to_csv(
    parquet: Path, csv_out: Path, duckdb_bin: str = DEFAULT_DUCKDB
) -> None:
    """COPY parquet → CSV with normalised column-name headers.

    Uses an explicit projection `"orig" AS "clean"` so the CSV header
    contains whitespace-stripped, dedup'd names. See
    `_normalise_column_names` for the rules.
    """
    import pyarrow.parquet as pq  # type: ignore

    schema = pq.read_schema(str(parquet))
    originals = list(schema.names)
    cleaned = _normalise_column_names(originals)
    projection = ", ".join(
        f'{_sql_ident(o)} AS {_sql_ident(c)}'
        for o, c in zip(originals, cleaned)
    ) or "*"
    sql = (
        f"COPY (SELECT {projection} FROM read_parquet({_sql_str(str(parquet))})) "
        f"TO {_sql_str(str(csv_out))} (HEADER, DELIMITER ',', QUOTE '\"');"
    )
    res = subprocess.run(
        [duckdb_bin, "-c", sql],
        capture_output=True, text=True, timeout=60,
    )
    if res.returncode != 0:
        raise RuntimeError(
            f"duckdb parquet→csv failed (rc={res.returncode}): {res.stderr.strip()}"
        )


def _sql_ident(s: str) -> str:
    """SQL double-quoted identifier."""
    return '"' + s.replace('"', '""') + '"'


def _sql_str(s: str) -> str:
    """SQL single-quoted string literal."""
    return "'" + s.replace("'", "''") + "'"


def _profile(csv_path: Path, schema_out: Path, finetype_bin: str) -> None:
    res = subprocess.run(
        [finetype_bin, "profile", "-f", str(csv_path), "-o", "json-schema"],
        capture_output=True, text=True, timeout=120,
    )
    if res.returncode != 0:
        raise RuntimeError(
            f"finetype profile failed (rc={res.returncode}): {res.stderr.strip()[-400:]}"
        )
    schema_out.write_text(res.stdout)


def _validate(
    csv_path: Path, schema_path: Path, db_out: Path, finetype_bin: str
) -> dict:
    res = subprocess.run(
        [
            finetype_bin, "validate",
            str(csv_path), str(schema_path),
            "--db", str(db_out), "--table", "T",
            "--lenient", "-o", "json",
        ],
        capture_output=True, text=True, timeout=180,
    )
    if res.returncode != 0:
        raise RuntimeError(
            f"finetype validate failed (rc={res.returncode}): {res.stderr.strip()[-400:]}"
        )
    return json.loads(res.stdout)


def _column_types(schema_path: Path) -> dict[str, str]:
    """Returns {column_name: x-finetype-label}."""
    schema = json.loads(schema_path.read_text())
    out: dict[str, str] = {}
    props = schema.get("properties", {})
    for name, defn in props.items():
        if isinstance(defn, dict):
            out[name] = defn.get("x-finetype-label", "")
    return out


def _distinct_rejected_rows_in(
    db_path: Path, duckdb_bin: str, columns: set[str]
) -> int:
    """Count distinct CSV lines (rows) that had ≥1 reject in `columns`.

    Contract criterion is "≤ 1% of rows rejected on the non-trivial
    columns" — per-row, not per-(row, column) cell. Summing per-column
    counts over-counts rows that fail in multiple columns; the right
    primitive is `COUNT(DISTINCT line) WHERE column_name IN columns`.
    """
    if not columns:
        return 0
    in_list = ",".join(_sql_str(c) for c in sorted(columns))
    sql = (
        f"SELECT COUNT(DISTINCT line) FROM finetype_reject_errors "
        f"WHERE column_name IN ({in_list});"
    )
    res = subprocess.run(
        [duckdb_bin, "-noheader", "-csv", str(db_path), sql],
        capture_output=True, text=True, timeout=30,
    )
    if res.returncode != 0:
        raise RuntimeError(
            f"reject-sidecar query failed: {res.stderr.strip()[-400:]}"
        )
    out = res.stdout.strip()
    return int(out) if out.lstrip("-").isdigit() else 0


def _measure_one(
    parquet: Path,
    *,
    finetype_bin: str = DEFAULT_FINETYPE,
    duckdb_bin: str = DEFAULT_DUCKDB,
) -> FileOutcome:
    t0 = time.perf_counter()
    with tempfile.TemporaryDirectory(prefix="finetype-gate-") as td:
        tmp = Path(td)
        csv_path = tmp / "in.csv"
        schema_path = tmp / "in.schema.json"
        db_path = tmp / "in.db"
        try:
            _parquet_to_csv(parquet, csv_path, duckdb_bin)
            _profile(csv_path, schema_path, finetype_bin)
            summary = _validate(csv_path, schema_path, db_path, finetype_bin)
            col_types = _column_types(schema_path)
            n_cols = len(col_types)
            non_trivial = {
                c for c, t in col_types.items() if t and t not in TRIVIAL_TYPES
            }
            non_trivial_cols = len(non_trivial)
            total_rows = int(summary.get("total_rows", 0))
            rejects_nt = (
                _distinct_rejected_rows_in(db_path, duckdb_bin, non_trivial)
                if n_cols else 0
            )
            non_trivial_pct = (non_trivial_cols / n_cols) if n_cols else 0.0
            reject_rate_nt = (rejects_nt / total_rows) if total_rows else 0.0
            passed = (
                n_cols > 0
                and non_trivial_pct >= NON_TRIVIAL_FLOOR
                and reject_rate_nt <= REJECT_RATE_CEIL
            )
            return FileOutcome(
                path=str(parquet),
                passed=passed,
                n_cols=n_cols,
                non_trivial_cols=non_trivial_cols,
                non_trivial_pct=round(non_trivial_pct, 4),
                total_rows=total_rows,
                rejects_non_trivial=rejects_nt,
                reject_rate_non_trivial=round(reject_rate_nt, 6),
                elapsed_s=round(time.perf_counter() - t0, 3),
            )
        except Exception as exc:  # noqa: BLE001
            return FileOutcome(
                path=str(parquet),
                passed=False,
                error=f"{type(exc).__name__}: {exc}"[:400],
                elapsed_s=round(time.perf_counter() - t0, 3),
            )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="GitTables holdout round-trip gate measurement"
    )
    parser.add_argument("--holdout", type=Path, default=DEFAULT_HOLDOUT)
    parser.add_argument("--model-dir", type=Path, default=DEFAULT_MODEL_DIR)
    parser.add_argument("--finetype-bin", default=DEFAULT_FINETYPE)
    parser.add_argument("--duckdb-bin", default=DEFAULT_DUCKDB)
    parser.add_argument(
        "--jobs", type=int, default=1,
        help="Parallel worker count (ProcessPoolExecutor). Default 1.",
    )
    parser.add_argument(
        "--max-files", type=int, default=None,
        help="Cap files measured (smoke test only).",
    )
    parser.add_argument(
        "--output", type=Path, default=None,
        help="Write summary JSON here in addition to stdout.",
    )
    parser.add_argument(
        "--no-per-file", action="store_true",
        help="Omit per-file outcomes from the JSON summary.",
    )
    parser.add_argument(
        "--quiet", action="store_true",
        help="Suppress per-file progress on stderr.",
    )
    args = parser.parse_args()

    # Pre-flight binary checks — fail loud if finetype/duckdb aren't on PATH.
    for label, candidate in (("finetype", args.finetype_bin), ("duckdb", args.duckdb_bin)):
        if shutil.which(candidate) is None and not Path(candidate).exists():
            print(f"error: {label} binary not found: {candidate}", file=sys.stderr)
            return 2

    paths = _read_holdout(args.holdout)
    if args.max_files is not None:
        paths = paths[: args.max_files]
    if not paths:
        print(f"error: no holdout paths in {args.holdout}", file=sys.stderr)
        return 2

    tag, weights_file, model_sha = _model_identity(args.model_dir)
    harness_sha, harness_dirty = _git_head(REPO_ROOT)

    if not args.quiet:
        print(
            f"gate: {len(paths)} files | model={tag} sha={model_sha[:12]} "
            f"| harness={harness_sha[:12]}{' DIRTY' if harness_dirty else ''}",
            file=sys.stderr,
        )

    t_start = time.perf_counter()
    outcomes: list[FileOutcome] = []
    if args.jobs <= 1:
        for i, p in enumerate(paths, 1):
            outcome = _measure_one(
                p,
                finetype_bin=args.finetype_bin,
                duckdb_bin=args.duckdb_bin,
            )
            outcomes.append(outcome)
            if not args.quiet and (i % 50 == 0 or i == len(paths)):
                passed = sum(1 for o in outcomes if o.passed)
                rate = i / (time.perf_counter() - t_start)
                print(
                    f"  {i}/{len(paths)} ({passed} pass, {rate:.1f} files/s)",
                    file=sys.stderr,
                )
    else:
        with cf.ProcessPoolExecutor(max_workers=args.jobs) as pool:
            futures = {
                pool.submit(
                    _measure_one, p,
                    finetype_bin=args.finetype_bin,
                    duckdb_bin=args.duckdb_bin,
                ): p
                for p in paths
            }
            for i, fut in enumerate(cf.as_completed(futures), 1):
                outcomes.append(fut.result())
                if not args.quiet and (i % 50 == 0 or i == len(paths)):
                    passed = sum(1 for o in outcomes if o.passed)
                    rate = i / (time.perf_counter() - t_start)
                    print(
                        f"  {i}/{len(paths)} ({passed} pass, {rate:.1f} files/s)",
                        file=sys.stderr,
                    )

    elapsed = time.perf_counter() - t_start
    files_passed = sum(1 for o in outcomes if o.passed)
    files_total = len(outcomes)
    gate_score = (files_passed / files_total) if files_total else 0.0
    n_errors = sum(1 for o in outcomes if o.error)

    summary = {
        "gate_score": round(gate_score, 6),
        "files_passed": files_passed,
        "files_total": files_total,
        "files_errored": n_errors,
        "model_sha": model_sha,
        "model_tag": tag,
        "model_weights_filename": weights_file,
        "harness_sha": harness_sha,
        "harness_dirty": harness_dirty,
        "holdout_path": str(args.holdout),
        "elapsed_s": round(elapsed, 3),
        "non_trivial_floor": NON_TRIVIAL_FLOOR,
        "reject_rate_ceil": REJECT_RATE_CEIL,
    }
    if not args.no_per_file:
        summary["per_file"] = [asdict(o) for o in outcomes]

    text = json.dumps(summary, indent=2)
    print(text)
    if args.output is not None:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text + "\n")

    if not args.quiet:
        print(
            f"gate_score: {gate_score:.4f} ({files_passed}/{files_total}) "
            f"errors={n_errors} elapsed={elapsed:.1f}s",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
