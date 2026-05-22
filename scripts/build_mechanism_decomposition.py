#!/usr/bin/env python3
"""ac-08 — per-criterion failure decomposition by the mechanism cascade.

Reads:
  - eval/gittables/corpus_pass/files.parquet      (criterion classification)
  - eval/gittables/corpus_pass/columns.parquet    (Sense predictions, samples)
  - eval/gittables/corpus_pass/per_column_rejects.parquet  (criterion-b side)

For each (file, column) that contributes to a criterion failure, calls
`finetype infer --mode column --batch --explain` (NDJSON stream — model +
taxonomy load once for the whole batch) to attribute one of the 10 closed
mechanism tokens. Applies the spec's locked mechanism → action-class
mapping, then writes:

  eval/gittables/corpus_pass/mechanism_decomposition.parquet
    columns: file_path, criterion, column_name, mechanism_token,
             recommended_action_class,
             contributing_columns_count_or_reject_count

`prediction_confirmed` rows are skipped (the mapping defines them as
N/A — no gap is surfaced).

USAGE
    source eval/gittables/.venv/bin/activate
    python3 scripts/build_mechanism_decomposition.py
    # optional: --a-only  (test the (a)-side without per-column-rejects)
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

NON_TRIVIAL_FLOOR = 0.80
REJECT_RATE_CEIL = 0.01

# Spec-locked mapping. `prediction_confirmed` → None (skip row).
MECHANISM_TO_ACTION = {
    "format_diversity_path_a":  "validator_widening",
    "format_diversity_path_b":  "model_retrain",
    "code_vs_canonical_path_a": "model_retrain",
    "code_vs_canonical_path_b": "model_retrain",
    "enum_overfit":             "validator_widening",
    "misclassification":        "training_data_addition",
    "prediction_confirmed":     None,
    "validator_widening":       "validator_widening",
    "unknown_no_fit":           "taxonomy_addition",
    "fallthrough":              "fallback_adjustment",
}
CLOSED_MECHANISMS = set(MECHANISM_TO_ACTION.keys())

SAMPLE_SEPARATOR = "│"  # U+2502 — what the corpus pass uses to join samples


def split_samples(s: str | None) -> list[str]:
    if not s:
        return []
    return [p for p in s.split(SAMPLE_SEPARATOR) if p]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--files-parquet", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/files.parquet")
    p.add_argument("--columns-parquet", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/columns.parquet")
    p.add_argument("--per-column-rejects", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/per_column_rejects.parquet")
    p.add_argument("--out", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/mechanism_decomposition.parquet")
    p.add_argument("--finetype-bin", default="finetype")
    p.add_argument("--a-only", action="store_true",
                   help="Skip the (b)-side; useful for testing before "
                        "per_column_rejects.parquet exists.")
    p.add_argument("--limit", type=int, default=None,
                   help="Process at most N target (file, column) pairs "
                        "(testing only).")
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: dependency missing ({exc}).", file=sys.stderr)
        return 2

    # ── (1) Build target list via DuckDB ──────────────────────────────
    con = duckdb.connect()
    con.execute(f"""
        CREATE TEMP VIEW files AS
        SELECT
            file_path,
            (error IS NULL AND n_cols > 0
             AND CAST(non_trivial_cols AS DOUBLE) / n_cols < {NON_TRIVIAL_FLOOR}) AS fails_a,
            (error IS NULL AND total_rows > 0
             AND CAST(rejects_non_trivial AS DOUBLE) / total_rows > {REJECT_RATE_CEIL}) AS fails_b
        FROM read_parquet('{args.files_parquet}')
    """)

    # (a)-side targets: trivial columns of criterion-a-failing files
    print("building (a)-side target set...", file=sys.stderr)
    a_targets_df = con.execute(f"""
        SELECT c.file_path, c.column_name,
               c.sense_prediction, c.sample_values_truncated
        FROM read_parquet('{args.columns_parquet}') c
        JOIN files f USING (file_path)
        WHERE f.fails_a AND c.is_trivial
    """).fetch_arrow_table()
    print(f"  (a)-side: {a_targets_df.num_rows} target columns",
          file=sys.stderr)

    # (b)-side targets: columns with rejects in criterion-b-failing files
    b_targets_df = None
    if not args.a_only and args.per_column_rejects.exists():
        print("building (b)-side target set...", file=sys.stderr)
        b_targets_df = con.execute(f"""
            SELECT c.file_path, c.column_name,
                   c.sense_prediction, c.sample_values_truncated,
                   r.reject_count
            FROM read_parquet('{args.columns_parquet}') c
            JOIN files f USING (file_path)
            JOIN read_parquet('{args.per_column_rejects}') r
                 ON r.file_path = c.file_path AND r.column_name = c.column_name
            WHERE f.fails_b AND r.reject_count > 0
        """).fetch_arrow_table()
        print(f"  (b)-side: {b_targets_df.num_rows} target columns",
              file=sys.stderr)
    elif not args.a_only:
        print(f"warning: {args.per_column_rejects} not found; skipping (b)-side",
              file=sys.stderr)

    # ── (2) Build NDJSON input for finetype infer --explain ───────────
    # Unique (column_name, predicted_type, samples) keys → call once,
    # apply to every target row that shares the key. This dedupes the
    # cascade work across files whose columns happen to share the same
    # name + prediction + sample shape.
    print("building NDJSON input + dedup map...", file=sys.stderr)

    # Stream from arrow tables. Each row produces (key, target_descriptor).
    def iter_targets():
        for tbl, kind, has_reject in [
            (a_targets_df, "non_trivial_floor", False),
            (b_targets_df, "reject_rate_ceil", True),
        ]:
            if tbl is None:
                continue
            cols = {name: tbl.column(name).to_pylist() for name in tbl.column_names}
            n = tbl.num_rows
            for i in range(n):
                samples = split_samples(cols["sample_values_truncated"][i])
                yield {
                    "file_path": cols["file_path"][i],
                    "column_name": cols["column_name"][i],
                    "criterion": kind,
                    "predicted_type": cols["sense_prediction"][i] or "unknown",
                    "samples": samples,
                    "count": cols["reject_count"][i] if has_reject else 1,
                }

    # Dedup by (column_name, predicted_type, samples-tuple)
    cascade_keys: dict[tuple, dict] = {}
    targets: list[dict] = []
    for t in iter_targets():
        key = (t["column_name"], t["predicted_type"], tuple(t["samples"]))
        cascade_keys[key] = {
            "column_name": t["column_name"],
            "predicted_type": t["predicted_type"],
            "samples": t["samples"],
        }
        targets.append(t)
        if args.limit and len(targets) >= args.limit:
            break

    print(f"  total target rows: {len(targets)}", file=sys.stderr)
    print(f"  unique cascade keys: {len(cascade_keys)} "
          f"({len(cascade_keys)/max(1,len(targets))*100:.1f}% of targets)",
          file=sys.stderr)

    # ── (3) Run finetype infer --mode column --batch --explain ────────
    # Write inputs to a temp file, then stream them to finetype's stdin.
    # The output is NDJSON, one line per input, in the same order.
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".ndjson", delete=False
    ) as inp_f:
        inp_path = Path(inp_f.name)
        # Stable iteration order for reproducibility
        cascade_items = list(cascade_keys.items())
        for _, payload in cascade_items:
            inp_f.write(json.dumps(payload) + "\n")
    out_path = Path(tempfile.mkstemp(suffix=".ndjson")[1])
    print(f"running finetype infer --explain on {len(cascade_items)} cascade inputs...",
          file=sys.stderr)
    t0 = time.perf_counter()
    with open(inp_path) as fin, open(out_path, "w") as fout:
        rc = subprocess.run(
            [args.finetype_bin, "infer", "--mode", "column",
             "--batch", "--explain"],
            stdin=fin, stdout=fout, check=False,
        ).returncode
    elapsed = time.perf_counter() - t0
    if rc != 0:
        print(f"error: finetype infer --explain exited {rc}", file=sys.stderr)
        return rc
    print(f"  cascade done in {elapsed:.1f}s "
          f"({len(cascade_items)/max(0.001,elapsed):.0f} columns/s)",
          file=sys.stderr)

    # ── (4) Parse cascade output, build key → mechanism map ───────────
    mechanism_by_key: dict[tuple, str] = {}
    with open(out_path) as f:
        for (key, _), line in zip(cascade_items, f):
            try:
                r = json.loads(line)
            except json.JSONDecodeError:
                continue
            mechanism = r.get("mechanism")
            if mechanism in CLOSED_MECHANISMS:
                mechanism_by_key[key] = mechanism

    inp_path.unlink(missing_ok=True)
    out_path.unlink(missing_ok=True)

    # ── (5) Emit mechanism_decomposition.parquet ──────────────────────
    print(f"writing {args.out}...", file=sys.stderr)
    schema = pa.schema([
        ("file_path", pa.string()),
        ("criterion", pa.string()),
        ("column_name", pa.string()),
        ("mechanism_token", pa.string()),
        ("recommended_action_class", pa.string()),
        ("contributing_columns_count_or_reject_count", pa.int64()),
    ])
    args.out.parent.mkdir(parents=True, exist_ok=True)
    n_emitted = n_skipped_confirmed = n_skipped_unmapped = 0
    fp_b, cr_b, cn_b, mt_b, ra_b, ct_b = [], [], [], [], [], []
    BUF = 100_000
    writer = pq.ParquetWriter(args.out, schema, compression="snappy")

    def _flush():
        nonlocal fp_b, cr_b, cn_b, mt_b, ra_b, ct_b
        if not fp_b:
            return
        writer.write_table(pa.table({
            "file_path": fp_b, "criterion": cr_b, "column_name": cn_b,
            "mechanism_token": mt_b, "recommended_action_class": ra_b,
            "contributing_columns_count_or_reject_count": ct_b,
        }, schema=schema))
        fp_b, cr_b, cn_b, mt_b, ra_b, ct_b = [], [], [], [], [], []

    for t in targets:
        key = (t["column_name"], t["predicted_type"], tuple(t["samples"]))
        mech = mechanism_by_key.get(key)
        if mech is None:
            n_skipped_unmapped += 1
            continue
        action = MECHANISM_TO_ACTION[mech]
        if action is None:  # prediction_confirmed
            n_skipped_confirmed += 1
            continue
        fp_b.append(t["file_path"])
        cr_b.append(t["criterion"])
        cn_b.append(t["column_name"])
        mt_b.append(mech)
        ra_b.append(action)
        ct_b.append(int(t["count"]))
        n_emitted += 1
        if len(fp_b) >= BUF:
            _flush()
    _flush()
    writer.close()

    print(json.dumps({
        "n_targets": len(targets),
        "n_unique_cascade_keys": len(cascade_keys),
        "n_emitted_rows": n_emitted,
        "n_skipped_prediction_confirmed": n_skipped_confirmed,
        "n_skipped_unmapped_or_missing": n_skipped_unmapped,
        "cascade_elapsed_seconds": round(elapsed, 1),
        "output_parquet": str(args.out),
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
