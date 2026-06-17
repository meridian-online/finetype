#!/usr/bin/env python3
"""ac-00: build the categorical-boundary re-adjudication worklist
(spec 2026-06-17-cardinality-boundary-readjudication).

For each boundary column (gold == categorical OR a type the model confuses with
it), emit the TRUE full-column signal the original 8-25-value labelling never had:
distinct count, row count, cardinality ratio, and the top distinct values WITH
their counts (the repetition structure that defines a categorical). One JSONL row
per column — the adjudication input for the ac-01 panel.

Run: eval/gittables/.venv/bin/python scripts/build_boundary_worklist.py
"""
import csv
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sweep_decisive_stats import resolve, con  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus.tsv")
PRED = os.path.join(REPO, "output/categorical-alnum-recall/predictions_baseline.tsv")
OUT = os.path.join(REPO, "output/cardinality-readjudication/worklist.jsonl")

CAT = "representation.discrete.categorical"
SPECIFIC = {
    "geography.location.city", "geography.location.region",
    "geography.transportation.iata_code", "geography.location.country_code",
    "representation.text.word", "representation.text.entity_name",
    "representation.text.plain_text", "representation.discrete.ordinal",
}


def rel(p):
    return (f"read_parquet('{p}')" if p.endswith(".parquet")
            else f"read_csv_auto('{p}', SAMPLE_SIZE=-1, ignore_errors=true)")


def value_counts(path, col, k=20):
    """Top-k distinct values with counts + (distinct, rows). None on failure."""
    q = col.replace('"', '""')
    src = rel(path)
    try:
        rows = con.execute(f'SELECT count(*), count(DISTINCT "{q}") FROM {src}').fetchone()
        n, nd = rows[0], rows[1]
        if not n:
            return None
        top = con.execute(
            f'SELECT "{q}" AS v, count(*) AS c FROM {src} WHERE "{q}" IS NOT NULL '
            f'GROUP BY 1 ORDER BY c DESC LIMIT {k}'
        ).fetchall()
        return n, nd, [(str(v), int(c)) for v, c in top]
    except Exception:
        return None


def main():
    pred = {(r["file_content_sha256"], r["column_name"]): r["predicted_label"]
            for r in csv.DictReader(open(PRED), delimiter="\t")}
    gold = list(csv.DictReader(open(GOLD), delimiter="\t"))
    os.makedirs(os.path.dirname(OUT), exist_ok=True)

    n_written = n_unresolved = 0
    with open(OUT, "w") as fh:
        for r in gold:
            if r["curated_label"] != CAT and r["curated_label"] not in SPECIFIC:
                continue
            k = (r["file_content_sha256"], r["column_name"])
            p = resolve(r["file_path"])
            vc = value_counts(p, r["column_name"]) if p else None
            if vc is None:
                # retry with exact column name from schema
                if p:
                    try:
                        schema = [c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel(p)}").fetchall()]
                        m = {str(c).strip().lower(): c for c in schema}
                        cn = m.get(r["column_name"].strip().lower())
                        vc = value_counts(p, cn) if cn else None
                    except Exception:
                        vc = None
            if vc is None:
                n_unresolved += 1
                continue
            n, nd, top = vc
            fh.write(json.dumps({
                "sha": r["file_content_sha256"],
                "header": r["column_name"],
                "current_gold": r["curated_label"],
                "model_pred": pred.get(k, ""),
                "rows": n,
                "distinct": nd,
                "card_ratio": round(nd / n, 4) if n else None,
                "top_values": top,
            }) + "\n")
            n_written += 1
    print(f"worklist: {n_written} columns -> {OUT}  ({n_unresolved} unresolved)")


if __name__ == "__main__":
    main()
