#!/usr/bin/env python3
"""ac-09 — two-lens AND corroboration over candidate gaps.

Reads ac-08's mechanism_decomposition.parquet plus columns.parquet
(Sense predictions + YDF predictions + sample values), groups columns
into candidate gaps by `(mechanism, taxonomy_signature,
value_shape_signature)`, and emits only the clusters where BOTH lenses
(YDF and cascade) flag a disagreement with Sense.

Per design revision 2026-05-21, DBpedia is NOT a corroboration lens;
the pool is exactly `{ydf, cascade}` and the filter is AND over those
two.

Outputs:
  eval/gittables/corpus_pass/corroborated_gaps.parquet
    schema: gap_id, criterion, mechanism, affected_column_count,
            recommended_action_class, rank_within_cell,
            corroborating_lenses (list<struct>),
            sample_evidence (list<struct>),
            candidate_spec_slug,
            safety_score [appended by post-step — see below]

Post-step (spec 2026-05-31-reachability-safety-score):
  After this script writes corroborated_gaps.parquet, run
    scripts/compute_cluster_safety_score.py
    scripts/augment_corroborated_gaps_with_safety.py
  to append the v3 `safety_score` advisory column. The augmentation
  is idempotent (drops existing safety_score before re-joining).
  CLAUDE.md's training_data_addition guidance assumes the column is
  present; regenerations of corroborated_gaps.parquet must re-run
  the post-step to preserve the diagnostic surface.
  eval/gittables/corpus_pass/single_lens_signals.tsv
    audit trail of columns where exactly one lens flagged

USAGE
    source eval/gittables/.venv/bin/activate
    python3 scripts/build_corroborated_gaps.py
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

YDF_CONFIDENCE_FLOOR = 0.5
SAMPLE_SEPARATOR = "│"  # U+2502 — corpus pass's sample join character
MIN_SAMPLE_EVIDENCE_ROWS = 3

# Spec-locked mapping. Same lookup ac-08 uses; duplicated here to keep
# ac-09 self-contained.
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


def char_class(c: str) -> str:
    """[A-Z]→A, [a-z]→a, [0-9]→9, other→. per ac-09 spec."""
    if "A" <= c <= "Z":
        return "A"
    if "a" <= c <= "z":
        return "a"
    if "0" <= c <= "9":
        return "9"
    return "."


def value_pattern(value: str) -> str:
    return "".join(char_class(c) for c in value)


def value_shape_signature(samples: list[str]) -> str:
    """SHA256 of sorted distinct character-class patterns from samples.

    With OBSERVED_SAMPLE_LIMIT=8 the pattern set has ≤8 elements
    before dedup. Coarseness is acceptable per ac-09 — higher
    affected_column_count per cluster, shorter top-N report.
    """
    patterns = sorted({value_pattern(v) for v in samples if v})
    h = hashlib.sha256()
    h.update("\n".join(patterns).encode("utf-8"))
    return h.hexdigest()


def split_samples(s: str | None) -> list[str]:
    if not s:
        return []
    return [p for p in s.split(SAMPLE_SEPARATOR) if p]


def gap_id_for(criterion: str, mechanism: str,
               file_col_pairs: list[tuple[str, str]]) -> str:
    """SHA256 of (criterion, mechanism, sorted affected-column signature)."""
    h = hashlib.sha256()
    h.update(f"{criterion}\x00{mechanism}\x00".encode("utf-8"))
    for fp, cn in sorted(set(file_col_pairs)):
        h.update(f"{fp}\x00{cn}\x00".encode("utf-8"))
    return h.hexdigest()


@dataclass
class Row:
    file_path: str
    column_name: str
    criterion: str
    mechanism: str
    sense_prediction: str
    ydf_prediction: str | None
    ydf_confidence: float | None
    samples: list[str]


def _ydf_flags(row: Row) -> bool:
    """YDF flags iff confidence >= floor AND its top-1 disagrees with Sense.

    YDF labels outside the current FineType taxonomy still count as
    disagreement — the disagreement itself is the signal regardless of
    whether YDF's specific guess is in-vocabulary. See ac-09 open
    question in the scoping doc.
    """
    if row.ydf_confidence is None or row.ydf_prediction is None:
        return False
    if row.ydf_confidence < YDF_CONFIDENCE_FLOOR:
        return False
    return row.ydf_prediction != row.sense_prediction


def _cascade_flags(row: Row) -> bool:
    """Cascade flags iff its mechanism is NOT prediction_confirmed.

    ac-08 already filters prediction_confirmed out of the input parquet,
    so in practice every row reaching ac-09 has the cascade flagging.
    The explicit check preserves correctness if upstream behaviour
    changes.
    """
    return row.mechanism != "prediction_confirmed"


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns-parquet", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/columns.parquet")
    p.add_argument("--mechanism-decomposition", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/mechanism_decomposition.parquet")
    p.add_argument("--out-corroborated", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet")
    p.add_argument("--out-single-lens", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/single_lens_signals.tsv")
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: dependency missing ({exc}).", file=sys.stderr)
        return 2

    if not args.mechanism_decomposition.exists():
        print(
            f"error: {args.mechanism_decomposition} not found — run "
            f"scripts/build_mechanism_decomposition.py first.",
            file=sys.stderr,
        )
        return 2

    # ── (1) Join per (file_path, column_name) ─────────────────────────
    print("joining columns × mechanism_decomposition...", file=sys.stderr)
    con = duckdb.connect()
    joined = con.execute(f"""
        SELECT
            m.file_path,
            m.column_name,
            m.criterion,
            m.mechanism_token AS mechanism,
            c.sense_prediction,
            c.ydf_prediction,
            c.ydf_confidence,
            c.sample_values_truncated
        FROM read_parquet('{args.mechanism_decomposition}') m
        JOIN read_parquet('{args.columns_parquet}') c
             USING (file_path, column_name)
    """).fetch_arrow_table()
    n_joined = joined.num_rows
    print(f"  joined rows: {n_joined}", file=sys.stderr)
    if n_joined == 0:
        print("error: join produced 0 rows — check inputs.", file=sys.stderr)
        return 2

    # ── (2) Lens votes per row ────────────────────────────────────────
    print("computing lens votes...", file=sys.stderr)
    cols = {n: joined.column(n).to_pylist() for n in joined.column_names}
    n_both = n_ydf_only = n_cascade_only = n_neither = 0

    # corroborated: keyed by (criterion, mechanism, sense_pred, value_shape)
    # value: list of Row
    clusters: dict[tuple, list[Row]] = defaultdict(list)
    single_lens_rows: list[dict] = []

    for i in range(n_joined):
        samples = split_samples(cols["sample_values_truncated"][i])
        row = Row(
            file_path=cols["file_path"][i],
            column_name=cols["column_name"][i],
            criterion=cols["criterion"][i],
            mechanism=cols["mechanism"][i],
            sense_prediction=cols["sense_prediction"][i] or "unknown",
            ydf_prediction=cols["ydf_prediction"][i],
            ydf_confidence=cols["ydf_confidence"][i],
            samples=samples,
        )
        ydf_f = _ydf_flags(row)
        cascade_f = _cascade_flags(row)
        if ydf_f and cascade_f:
            n_both += 1
            shape = value_shape_signature(row.samples)
            key = (row.criterion, row.mechanism, row.sense_prediction, shape)
            clusters[key].append(row)
        elif ydf_f and not cascade_f:
            n_ydf_only += 1
            single_lens_rows.append({
                "file_path": row.file_path,
                "column_name": row.column_name,
                "sense_prediction": row.sense_prediction,
                "ydf_prediction": row.ydf_prediction or "",
                "ydf_confidence": f"{row.ydf_confidence:.4f}"
                                  if row.ydf_confidence is not None else "",
                "mechanism": row.mechanism,
                "ydf_flagged": "1",
                "cascade_flagged": "0",
            })
        elif cascade_f and not ydf_f:
            n_cascade_only += 1
            single_lens_rows.append({
                "file_path": row.file_path,
                "column_name": row.column_name,
                "sense_prediction": row.sense_prediction,
                "ydf_prediction": row.ydf_prediction or "",
                "ydf_confidence": f"{row.ydf_confidence:.4f}"
                                  if row.ydf_confidence is not None else "",
                "mechanism": row.mechanism,
                "ydf_flagged": "0",
                "cascade_flagged": "1",
            })
        else:
            n_neither += 1

    print(f"  both-flag (corroborated): {n_both}", file=sys.stderr)
    print(f"  ydf-only (single-lens):   {n_ydf_only}", file=sys.stderr)
    print(f"  cascade-only:             {n_cascade_only}", file=sys.stderr)
    print(f"  neither (dropped):        {n_neither}", file=sys.stderr)

    # ── (3) Emit single_lens_signals.tsv ──────────────────────────────
    args.out_single_lens.parent.mkdir(parents=True, exist_ok=True)
    with open(args.out_single_lens, "w", newline="") as f:
        if single_lens_rows:
            writer = csv.DictWriter(
                f,
                fieldnames=list(single_lens_rows[0].keys()),
                delimiter="\t",
            )
            writer.writeheader()
            writer.writerows(single_lens_rows)
        else:
            # Write header anyway so the file is valid + greppable
            f.write(
                "file_path\tcolumn_name\tsense_prediction\tydf_prediction\t"
                "ydf_confidence\tmechanism\tydf_flagged\tcascade_flagged\n"
            )

    # ── (4) Build GapEntry per cluster, rank, write parquet ───────────
    print(f"emitting {len(clusters)} corroborated clusters to "
          f"{args.out_corroborated}...", file=sys.stderr)

    # First pass: build all entries
    entries = []
    for (criterion, mechanism, _sense_pred, _shape), rows in clusters.items():
        file_col_pairs = [(r.file_path, r.column_name) for r in rows]
        gid = gap_id_for(criterion, mechanism, file_col_pairs)
        action = MECHANISM_TO_ACTION.get(mechanism)
        if action is None:
            continue  # prediction_confirmed — but cascade-flag filter excludes already
        affected = len({(r.file_path, r.column_name) for r in rows})

        # corroborating_lenses: take a representative row (the first).
        # Each cluster shares (mechanism, sense_prediction, shape) so the
        # cascade vote is the same for every row; YDF's vote may vary
        # per-row but for the lens record we use the cluster representative.
        rep = rows[0]
        corroborating_lenses = [
            {
                "lens_name": "ydf",
                "prediction_or_annotation": rep.ydf_prediction or "",
                "confidence": float(rep.ydf_confidence)
                              if rep.ydf_confidence is not None else 0.0,
            },
            {
                "lens_name": "cascade",
                "prediction_or_annotation": rep.mechanism,
                "confidence": 1.0,
            },
        ]

        # sample_evidence: at least 3 rows; cap at 10 to keep parquet small
        evidence_rows = rows[:max(MIN_SAMPLE_EVIDENCE_ROWS, min(10, len(rows)))]
        sample_evidence = [
            {
                "file_path": r.file_path,
                "column_name": r.column_name,
                "sample_values": r.samples,
                "sense_prediction": r.sense_prediction,
                "ydf_prediction": r.ydf_prediction or "",
                "dbpedia_annotation_if_present": "",
            }
            for r in evidence_rows
        ]

        entries.append({
            "gap_id": gid,
            "criterion": criterion,
            "mechanism": mechanism,
            "affected_column_count": affected,
            "recommended_action_class": action,
            "corroborating_lenses": corroborating_lenses,
            "sample_evidence": sample_evidence,
            "candidate_spec_slug": "",
            "_cell_key": (criterion, mechanism),  # for ranking
        })

    # Rank within (criterion, mechanism) cell:
    # ORDER BY affected_column_count DESC, gap_id ASC
    entries_by_cell: dict[tuple, list[dict]] = defaultdict(list)
    for e in entries:
        entries_by_cell[e["_cell_key"]].append(e)
    for cell_entries in entries_by_cell.values():
        cell_entries.sort(
            key=lambda e: (-e["affected_column_count"], e["gap_id"])
        )
        for rank, e in enumerate(cell_entries, start=1):
            e["rank_within_cell"] = rank

    final_entries = [e for cell in entries_by_cell.values() for e in cell]
    for e in final_entries:
        del e["_cell_key"]

    # Write parquet
    schema = pa.schema([
        ("gap_id", pa.string()),
        ("criterion", pa.string()),
        ("mechanism", pa.string()),
        ("affected_column_count", pa.int64()),
        ("recommended_action_class", pa.string()),
        ("rank_within_cell", pa.int32()),
        ("corroborating_lenses", pa.list_(pa.struct([
            ("lens_name", pa.string()),
            ("prediction_or_annotation", pa.string()),
            ("confidence", pa.float64()),
        ]))),
        ("sample_evidence", pa.list_(pa.struct([
            ("file_path", pa.string()),
            ("column_name", pa.string()),
            ("sample_values", pa.list_(pa.string())),
            ("sense_prediction", pa.string()),
            ("ydf_prediction", pa.string()),
            ("dbpedia_annotation_if_present", pa.string()),
        ]))),
        ("candidate_spec_slug", pa.string()),
    ])

    if final_entries:
        table = pa.Table.from_pylist(final_entries, schema=schema)
    else:
        table = pa.Table.from_pylist([], schema=schema)
    args.out_corroborated.parent.mkdir(parents=True, exist_ok=True)
    pq.write_table(table, args.out_corroborated, compression="snappy")

    print(json.dumps({
        "n_joined_rows": n_joined,
        "n_both_flag": n_both,
        "n_ydf_only": n_ydf_only,
        "n_cascade_only": n_cascade_only,
        "n_neither_dropped": n_neither,
        "n_corroborated_gaps": len(final_entries),
        "n_single_lens_rows": len(single_lens_rows),
        "output_corroborated": str(args.out_corroborated),
        "output_single_lens": str(args.out_single_lens),
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
