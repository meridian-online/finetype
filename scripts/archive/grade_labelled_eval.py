#!/usr/bin/env python3
"""ac-11 — grade the diagnostic against labelled_eval.tsv.

A row is FLAGGED BY THE DIAGNOSTIC iff its (file_path, column_name)
pair appears as sample_evidence in any GapEntry that entered
corroborated_gaps.parquet (i.e. it's part of a corroborated cluster
per ac-09).

For each flagged row, emits a 10-field tuple. Precision is computed
on `mechanism_correct` only — the diagnostic's primary claim is
correct gap classification. `ydf_accuracy_on_flagged` is reported
alongside as a lens-quality stat (NOT in the threshold).

Outputs:
  eval/gittables/corpus_pass/labelled_eval_per_row.tsv
  eval/gittables/corpus_pass/labelled_eval_validation.json

Asserts: precision_on_flagged >= 0.80.

USAGE
    python3 scripts/grade_labelled_eval.py
"""
from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
PRECISION_FLOOR = 0.80

# Locked mechanism → recommended_action_class mapping.
# prediction_confirmed → None (no gap surfaced; not in the corroborated set).
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


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument(
        "--labelled-eval", type=Path,
        default=REPO / ".orbit/specs/2026-05-04-autonomous-type-inference/labelled_eval.tsv",
    )
    p.add_argument(
        "--corroborated", type=Path,
        default=REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet",
    )
    p.add_argument(
        "--dbpedia-annotations", type=Path,
        default=REPO / "eval/gittables/corpus_pass/dbpedia_annotations.parquet",
    )
    p.add_argument(
        "--out-tsv", type=Path,
        default=REPO / "eval/gittables/corpus_pass/labelled_eval_per_row.tsv",
    )
    p.add_argument(
        "--out-json", type=Path,
        default=REPO / "eval/gittables/corpus_pass/labelled_eval_validation.json",
    )
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: duckdb missing ({exc})", file=sys.stderr)
        return 2

    if not args.labelled_eval.exists():
        print(f"error: {args.labelled_eval} missing", file=sys.stderr)
        return 2
    if not args.corroborated.exists():
        print(f"error: {args.corroborated} missing — run ac-09 first",
              file=sys.stderr)
        return 2

    # ── (1) Build flag map: (file_path, column_name) → gap context ────
    # If a labelled row matches multiple gaps, deterministic preference:
    # lowest rank_within_cell (the gap with the largest affected_column_count
    # in its cell — the diagnostic's most confident classification about
    # this row's mechanism).
    print("loading corroborated_gaps sample_evidence...", file=sys.stderr)
    con = duckdb.connect()
    sample_rows = con.execute(f"""
        SELECT
          s.file_path,
          s.column_name,
          g.mechanism,
          g.recommended_action_class,
          g.rank_within_cell,
          g.gap_id
        FROM read_parquet('{args.corroborated}') g,
             UNNEST(g.sample_evidence) AS t(s)
    """).fetchall()

    flag_map: dict[tuple[str, str], dict] = {}
    for fp, cn, mech, action, rank, gap_id in sample_rows:
        key = (fp, cn)
        existing = flag_map.get(key)
        if existing is None or rank < existing["rank_within_cell"]:
            flag_map[key] = {
                "mechanism": mech,
                "recommended_action_class": action,
                "rank_within_cell": rank,
                "gap_id": gap_id,
            }
    print(f"  {len(flag_map)} distinct (file, column) pairs in sample_evidence",
          file=sys.stderr)

    # ── (2) DBpedia annotation lookup (same as ac-10's enrichment) ────
    dbp_lookup: dict[tuple[str, str], str] = {}
    if args.dbpedia_annotations.exists():
        dbp_rows = con.execute(f"""
            SELECT file_path, column_name, dbpedia_semantic_class
            FROM read_parquet('{args.dbpedia_annotations}')
            WHERE dbpedia_semantic_class IS NOT NULL
              AND dbpedia_semantic_class != ''
        """).fetchall()
        dbp_lookup = {(r[0], r[1]): r[2] for r in dbp_rows}

    # ── (3) Walk labelled_eval, grade flagged rows ────────────────────
    n_rows_total = 0
    n_rows_excluded = 0  # rows whose truth_mechanism is outside the closed set
    per_row_records: list[dict] = []

    with args.labelled_eval.open(newline="") as f:
        reader = csv.DictReader(f, delimiter="\t")
        for row_id, row in enumerate(reader):
            n_rows_total += 1
            truth_mech = (row.get("truth_mechanism") or "").strip()
            if truth_mech and truth_mech not in MECHANISM_TO_ACTION:
                n_rows_excluded += 1
                continue

            fp = row.get("file_path") or ""
            cn = row.get("column_name") or ""
            flag = flag_map.get((fp, cn))
            if flag is None:
                continue  # unflagged — not part of any corroborated gap

            sense_pred = row.get("predicted_type") or ""
            ydf_pred = ""  # labelled_eval doesn't carry YDF predictions
            truth_inferred = (row.get("truth_inferred_type") or "").strip()
            mechanism_token = flag["mechanism"]
            rec_action = flag["recommended_action_class"]

            # mechanism_correct: action_class match (equivalent to direct
            # token match given the closed mapping; spec accepts either).
            truth_action = MECHANISM_TO_ACTION.get(truth_mech)
            mechanism_correct = (
                truth_action is not None and rec_action == truth_action
            )
            ydf_correct = bool(ydf_pred) and ydf_pred == truth_inferred

            per_row_records.append({
                "row_id": str(row_id),
                "sense_prediction": sense_pred,
                "ydf_prediction": ydf_pred,
                "dbpedia_annotation": dbp_lookup.get((fp, cn), ""),
                "mechanism_token": mechanism_token,
                "recommended_action_class": rec_action,
                "truth_inferred_type": truth_inferred,
                "truth_mechanism": truth_mech,
                "mechanism_correct": "1" if mechanism_correct else "0",
                "ydf_correct": "1" if ydf_correct else "0",
            })

    # ── (4) Emit per-row TSV ──────────────────────────────────────────
    args.out_tsv.parent.mkdir(parents=True, exist_ok=True)
    tsv_fields = [
        "row_id", "sense_prediction", "ydf_prediction", "dbpedia_annotation",
        "mechanism_token", "recommended_action_class",
        "truth_inferred_type", "truth_mechanism",
        "mechanism_correct", "ydf_correct",
    ]
    with args.out_tsv.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=tsv_fields, delimiter="\t")
        w.writeheader()
        for r in per_row_records:
            w.writerow(r)

    # ── (5) Emit aggregate JSON ───────────────────────────────────────
    n_flagged = len(per_row_records)
    n_mech_correct = sum(1 for r in per_row_records if r["mechanism_correct"] == "1")
    n_ydf_correct = sum(1 for r in per_row_records if r["ydf_correct"] == "1")
    precision = (n_mech_correct / n_flagged) if n_flagged else 0.0
    ydf_acc = (n_ydf_correct / n_flagged) if n_flagged else 0.0

    summary = {
        "n_rows_total": n_rows_total,
        "n_rows_excluded": n_rows_excluded,
        "n_flagged_by_diagnostic": n_flagged,
        "n_mechanism_correct": n_mech_correct,
        "precision_on_flagged": round(precision, 4),
        "ydf_accuracy_on_flagged": round(ydf_acc, 4),
        "precision_floor": PRECISION_FLOOR,
        "asserted_precision_on_flagged_passes": precision >= PRECISION_FLOOR,
    }
    args.out_json.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))

    if n_flagged == 0:
        print(
            "\nERROR: zero rows in labelled_eval are flagged by the "
            "diagnostic. Precision is undefined — ac-11 cannot close.",
            file=sys.stderr,
        )
        return 1
    if precision < PRECISION_FLOOR:
        print(
            f"\nERROR: precision_on_flagged {precision:.4f} < "
            f"{PRECISION_FLOOR}. ac-11 fails.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
