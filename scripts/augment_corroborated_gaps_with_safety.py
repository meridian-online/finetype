#!/usr/bin/env python3
"""ac-04 — Append safety_score column to corroborated_gaps.parquet.

Spec: `.orbit/specs/2026-05-31-reachability-safety-score/spec.yaml`

In-place augmentation: reads
  - eval/gittables/corpus_pass/corroborated_gaps.parquet
  - output/cluster-reachability/cluster_safety_scores.parquet
LEFT JOINs on gap_id, writes back to corroborated_gaps.parquet.

Original schema columns preserved; `safety_score` appended.
gap_ids with null safety_score in input (cluster too small or pool
floor unmet) carry NULL through to output.

Invocation:
  uv run --with duckdb scripts/augment_corroborated_gaps_with_safety.py
  uv run --with duckdb scripts/augment_corroborated_gaps_with_safety.py --dry-run
"""
from __future__ import annotations

import argparse
import shutil
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_GAPS = REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet"
DEFAULT_SAFETY = REPO / "output/cluster-reachability/cluster_safety_scores.parquet"


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--gaps", type=Path, default=DEFAULT_GAPS)
    p.add_argument("--safety", type=Path, default=DEFAULT_SAFETY)
    p.add_argument("--dry-run", action="store_true",
                   help="Write to a sibling .augmented.parquet file instead")
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc})", file=sys.stderr)
        return 2

    if not args.gaps.exists():
        print(f"error: gaps parquet missing at {args.gaps}", file=sys.stderr)
        return 2
    if not args.safety.exists():
        print(f"error: safety parquet missing at {args.safety}", file=sys.stderr)
        return 2

    con = duckdb.connect()

    # Verify gap_id row counts match before doing the write.
    gaps_count = con.execute(
        "SELECT count(*) FROM read_parquet(?)", [str(args.gaps)],
    ).fetchone()[0]
    safety_count = con.execute(
        "SELECT count(*) FROM read_parquet(?)", [str(args.safety)],
    ).fetchone()[0]
    if gaps_count != safety_count:
        print(f"warn: gaps has {gaps_count} rows, safety has {safety_count}; "
              f"LEFT JOIN will leave unmatched safety_score as NULL",
              file=sys.stderr)

    if "safety_score" in [
        col[0] for col in con.execute(
            "DESCRIBE SELECT * FROM read_parquet(?)", [str(args.gaps)]
        ).fetchall()
    ]:
        print(f"warn: {args.gaps} already has a safety_score column; "
              f"it will be overwritten by the join", file=sys.stderr)

    if args.dry_run:
        out_path = args.gaps.with_suffix(".augmented.parquet")
    else:
        # Write to temp and swap atomically.
        tmp = tempfile.NamedTemporaryFile(
            suffix=".parquet", dir=args.gaps.parent, delete=False,
        )
        tmp.close()
        out_path = Path(tmp.name)

    # Build the JOIN query with explicit column list to avoid struct
    # quoting issues; drop any existing safety_score so we don't double-
    # nest the column on re-augmentation.
    schema = con.execute(
        "DESCRIBE SELECT * FROM read_parquet(?)", [str(args.gaps)],
    ).fetchall()
    cols = [row[0] for row in schema if row[0] != "safety_score"]
    select_cols = ", ".join([f"g.{c}" for c in cols])
    query = (
        f"COPY (SELECT {select_cols}, s.safety_score "
        f"FROM read_parquet('{args.gaps}') g "
        f"LEFT JOIN read_parquet('{args.safety}') s "
        f"ON g.gap_id = s.gap_id "
        f"ORDER BY g.affected_column_count DESC, g.gap_id ASC) "
        f"TO '{out_path}' (FORMAT PARQUET)"
    )
    con.execute(query)

    # Verify output row count matches input.
    out_count = con.execute(
        "SELECT count(*) FROM read_parquet(?)", [str(out_path)],
    ).fetchone()[0]
    if out_count != gaps_count:
        print(f"error: output has {out_count} rows, expected {gaps_count}; "
              f"keeping output at {out_path} but NOT swapping",
              file=sys.stderr)
        return 1

    non_null_safety = con.execute(
        "SELECT count(*) FROM read_parquet(?) WHERE safety_score IS NOT NULL",
        [str(out_path)],
    ).fetchone()[0]
    print(f"output {out_path}: {out_count} rows, "
          f"{non_null_safety} with non-null safety_score",
          file=sys.stderr)

    if args.dry_run:
        print(f"dry-run: augmented file written to {out_path}, "
              f"original at {args.gaps} unchanged", file=sys.stderr)
        return 0

    # Atomic swap.
    shutil.move(str(out_path), str(args.gaps))
    print(f"swapped {args.gaps} (now has safety_score column)",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
