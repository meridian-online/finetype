#!/usr/bin/env python3
"""ac-04 — Per-cluster false-positive rate, v23 vs v22.

For each of the six target false-positive clusters from spec
2026-05-27-v23-precision-retrain, count the columns where Sense
fires the FP label AND YDF/cascade contradicts it (the cluster's
defining condition), and report the v22 → v23 drop.

Cluster definition: the corroborated-gaps cell of report.md filters
on `criterion = reject_rate_ceil`, `mechanism = misclassification`
— but at this point we don't re-run the cascade, we approximate
the cluster membership with the (sense_prediction, ydf_prediction)
pair from the corpus-pass `columns.parquet`. That mirrors how ac-01
extracted hard negatives in the first place.

Reads:
  output/corpus-pass-v22/corpus_pass/columns.parquet
  output/corpus-pass-v23/corpus_pass/columns.parquet

Writes:
  output/v23-precision-retrain/per_cluster_fp_rate.md
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
V22_COLUMNS = REPO / "output/corpus-pass-v22/corpus_pass/columns.parquet"
V23_COLUMNS = REPO / "output/corpus-pass-v23/corpus_pass/columns.parquet"
DEFAULT_OUT = REPO / "output/v23-precision-retrain/per_cluster_fp_rate.md"

# (cluster_id, FP_label, correct_label, expected_v22_cols_per_report)
# expected_v22_cols comes from eval/gittables/corpus_pass/report.md and
# is recorded for sanity-checking — if our recomputed v22 count drifts
# materially from the report, the cluster definition has changed.
TARGET_CLUSTERS = [
    ("721b890ea74d",
     "identity.person.gender_code",
     "representation.discrete.categorical", 22952),
    ("1b858e0d073b",
     "datetime.offset.utc",
     "representation.numeric.integer_number", 21956),
    ("20803deffbad",
     "technology.internet.url",
     "representation.numeric.integer_number", 9779),
    ("81b63a52e3ef",
     "representation.boolean.binary",
     "representation.numeric.integer_number", 8649),
    ("cdde5d05b73a",
     "datetime.component.periodicity",
     "representation.discrete.categorical", 5764),
    ("3f2aa8465552",
     "representation.identifier.alphanumeric_id",
     "representation.discrete.categorical", 4835),
]

# ac-04 band thresholds, pre-committed in the spec.
BAND_MET_DROP_PCT = 50.0
BAND_PARTIAL_DROP_PCT = 20.0


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--v22-columns", type=Path, default=V22_COLUMNS)
    p.add_argument("--v23-columns", type=Path, default=V23_COLUMNS)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    for path in (args.v22_columns, args.v23_columns):
        if not path.exists():
            print(f"error: missing {path}", file=sys.stderr)
            return 2

    con = duckdb.connect()

    # Per-cluster column counts for both versions. A column belongs to
    # a cluster if its (sense_prediction, ydf_prediction) matches the
    # cluster's (FP_label, correct_label) pair.
    per_cluster: dict[str, dict] = {}
    for cluster_id, fp_label, correct_label, expected_v22 in TARGET_CLUSTERS:
        v22_count = con.execute(f"""
            SELECT COUNT(*) FROM read_parquet('{args.v22_columns.as_posix()}')
             WHERE sense_prediction = ? AND ydf_prediction = ?
        """, [fp_label, correct_label]).fetchone()[0]
        v23_count = con.execute(f"""
            SELECT COUNT(*) FROM read_parquet('{args.v23_columns.as_posix()}')
             WHERE sense_prediction = ? AND ydf_prediction = ?
        """, [fp_label, correct_label]).fetchone()[0]
        drop = v22_count - v23_count
        drop_pct = (drop / v22_count * 100.0) if v22_count else 0.0
        per_cluster[cluster_id] = {
            "fp_label": fp_label,
            "correct_label": correct_label,
            "expected_v22": expected_v22,
            "v22": v22_count,
            "v23": v23_count,
            "drop": drop,
            "drop_pct": drop_pct,
        }
        print(f"  {cluster_id}: v22={v22_count:,} v23={v23_count:,} "
              f"(Δ −{drop:,}, −{drop_pct:.1f}%)", file=sys.stderr)

    total_v22 = sum(c["v22"] for c in per_cluster.values())
    total_v23 = sum(c["v23"] for c in per_cluster.values())
    total_drop = total_v22 - total_v23
    total_drop_pct = (total_drop / total_v22 * 100.0) if total_v22 else 0.0

    # Band verdict (FP-rate component only; the cell-2 component is
    # computed by cell_deltas_v23_vs_v22.md and combined there).
    if total_drop_pct >= BAND_MET_DROP_PCT:
        band_fp = "Met"
    elif total_drop_pct >= BAND_PARTIAL_DROP_PCT:
        band_fp = "Partial"
    else:
        band_fp = "Failed"

    lines: list[str] = []
    lines.append("# v23 vs v22 — per-cluster false-positive rate\n\n")
    lines.append("Per spec `2026-05-27-v23-precision-retrain` ac-04.\n\n")
    lines.append(
        "Each row counts columns in the gittables corpus pass where "
        "Sense fires the v22 FP label AND YDF disagrees with the "
        "cluster's correct label — the same `(sense_prediction, "
        "ydf_prediction)` pair the ac-01 extraction filtered on. The "
        "v22 numbers are recomputed here for ground truth; the "
        "`expected (report)` column comes from "
        "`eval/gittables/corpus_pass/report.md` and is kept for "
        "sanity-checking only.\n\n"
    )

    lines.append("## Per-cluster trajectory\n\n")
    lines.append(
        "| cluster_id | FP label (v22) | correct label (YDF) | expected (report) | v22 (recomputed) | v23 | Δ | Δ% |\n"
    )
    lines.append("|---|---|---|---:|---:|---:|---:|---:|\n")
    for cluster_id, fp_label, correct_label, expected in TARGET_CLUSTERS:
        c = per_cluster[cluster_id]
        sense_short = fp_label.replace("representation.", "rep.")
        ydf_short = correct_label.replace("representation.", "rep.")
        bold = "**" if c["drop_pct"] >= BAND_MET_DROP_PCT else ""
        lines.append(
            f"| `{cluster_id}…` | `{sense_short}` | `{ydf_short}` | "
            f"{expected:,} | {c['v22']:,} | {bold}{c['v23']:,}{bold} | "
            f"−{c['drop']:,} | {bold}−{c['drop_pct']:.1f}%{bold} |\n"
        )
    lines.append("\n")

    lines.append("## Totals\n\n")
    lines.append(f"- Total v22 FP columns across the six clusters: **{total_v22:,}**\n")
    lines.append(f"- Total v23 FP columns across the six clusters: **{total_v23:,}**\n")
    lines.append(f"- Drop: **−{total_drop:,} (−{total_drop_pct:.1f}%)**\n\n")

    lines.append("## ac-04 band — FP-rate component\n\n")
    lines.append(
        f"Pre-committed thresholds: Met ≥ {BAND_MET_DROP_PCT:.0f}%; "
        f"Partial ≥ {BAND_PARTIAL_DROP_PCT:.0f}%; Failed < "
        f"{BAND_PARTIAL_DROP_PCT:.0f}%.\n\n"
    )
    lines.append(f"**Verdict: {band_fp}** (−{total_drop_pct:.1f}% drop on "
                 "the top-6 cluster columns).\n\n")
    lines.append(
        "The final ac-04 band combines this FP-rate verdict with the "
        "gated cell-2 verdict (see `cell_deltas_v23_vs_v22.md`):\n"
        f"  - **Met** (ship as default) — FP drop ≥ {BAND_MET_DROP_PCT:.0f}% "
        "AND gated cell-2 vs v19 non-worse than v22's −10.4%.\n"
        f"  - **Partial** (review-and-decide) — FP drop "
        f"{BAND_PARTIAL_DROP_PCT:.0f}–{BAND_MET_DROP_PCT:.0f}% AND cell-2 "
        "within ±2pp of v22.\n"
        "  - **Failed** — FP drop below partial threshold OR cell-2 "
        "regresses ≥ 2pp vs v22.\n"
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(lines))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
