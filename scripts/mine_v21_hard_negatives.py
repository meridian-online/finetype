#!/usr/bin/env python3
"""ac-02 — Mine v21 corpus-pass output for hard-negative training rows.

Reads output/corpus-pass-v21/corpus_pass/columns.parquet and emits, in
the sherlock_distilled schema, columns the v21 Sense model classified
as geography but a high-confidence YDF lens disagrees with. The
training label is YDF's prediction — these are the "looks-like-city
but isn't" boundary examples v22 needs.

Filters (per spec 2026-05-25-v22-boundary-training ac-02):
  - sense_prediction LIKE 'geography.%'
  - ydf_prediction NOT LIKE 'geography.%'
  - ydf_confidence >= YDF_CONFIDENCE_FLOOR
  - ydf_prediction IN the FineType taxonomy
  - sample_values has at least --min-values entries

The YDF-confidence floor is the corroboration the spec asks for —
"the value sample passes the validation check for YDF's prediction
class". FineType has no Python-callable validator per type, so we use
high YDF confidence (≥0.7) as the per-row positive corroboration
proxy. With a 0.7 floor, the noisiest YDF predictions (e.g. the
0.51-confidence "Author → city" rows we saw in the m-19 corpus) are
filtered out.

Caps:
  - per-type: 2,000 (head off any single YDF-favourite swamping the
    blend; downstream prepare_multibranch_data applies its own caps)
  - per-sense-type: no cap — sense_prediction is by definition
    geography.* so all rows are city/region/etc., naturally bounded
    by the v21 corpus pass volume.

Output:
  output/distillation-v22/hard_negatives_mined.csv.gz
  per-label counts appended to
  output/distillation-v22/hard_negatives_manifest.json
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import random
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_COLUMNS = REPO / "output/corpus-pass-v21/corpus_pass/columns.parquet"
DEFAULT_OUT = REPO / "output/distillation-v22/hard_negatives_mined.csv.gz"
DEFAULT_MANIFEST = REPO / "output/distillation-v22/hard_negatives_manifest.json"
DEFAULT_TAXONOMY = REPO / "models/default/label_map.json"

YDF_CONFIDENCE_FLOOR = 0.7
PER_TYPE_CAP = 2000
SAMPLE_SEPARATOR = "│"


def split_samples(s: str | None) -> list[str]:
    if not s:
        return []
    return [p for p in s.split(SAMPLE_SEPARATOR) if p]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns", type=Path, default=DEFAULT_COLUMNS)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--taxonomy", type=Path, default=DEFAULT_TAXONOMY)
    p.add_argument("--min-values", type=int, default=5)
    p.add_argument("--seed", type=int, default=42)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    taxonomy = set(json.loads(args.taxonomy.read_text()))
    print(f"taxonomy: {len(taxonomy)} labels", file=sys.stderr)

    con = duckdb.connect()
    con.execute("CREATE OR REPLACE TEMP TABLE taxonomy(label VARCHAR)")
    con.executemany("INSERT INTO taxonomy VALUES (?)",
                    [(t,) for t in taxonomy])

    print(f"querying {args.columns}...", file=sys.stderr)
    arrow = con.execute(f"""
        SELECT column_name,
               sense_prediction,
               ydf_prediction,
               ydf_confidence,
               sample_values_truncated
          FROM read_parquet('{args.columns.as_posix()}')
         WHERE sense_prediction LIKE 'geography.%'
           AND ydf_prediction IS NOT NULL
           AND ydf_prediction NOT LIKE 'geography.%'
           AND ydf_confidence >= {YDF_CONFIDENCE_FLOOR}
           AND ydf_prediction IN (SELECT label FROM taxonomy)
    """).to_arrow_table()
    n_cand = arrow.num_rows
    print(f"  candidate rows: {n_cand:,}", file=sys.stderr)

    cols = {n: arrow.column(n).to_pylist() for n in arrow.column_names}

    # Shuffle for unbiased per-type capping (otherwise the first rows of
    # each type would dominate). Per-type cap is applied as we go.
    rng = random.Random(args.seed)
    order = list(range(n_cand))
    rng.shuffle(order)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    per_type_count: dict[str, int] = defaultdict(int)
    per_sense_count: dict[str, int] = defaultdict(int)
    per_type_cap_hits: dict[str, int] = defaultdict(int)
    n_written = 0
    n_skipped_sparse = 0

    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])
        for i in order:
            samples = split_samples(cols["sample_values_truncated"][i])
            if len(samples) < args.min_values:
                n_skipped_sparse += 1
                continue
            ydf_pred = cols["ydf_prediction"][i]
            if per_type_count[ydf_pred] >= PER_TYPE_CAP:
                per_type_cap_hits[ydf_pred] += 1
                continue
            writer.writerow([
                ydf_pred,
                json.dumps(samples, ensure_ascii=False),
                cols["column_name"][i] or "",
            ])
            per_type_count[ydf_pred] += 1
            per_sense_count[cols["sense_prediction"][i] or "?"] += 1
            n_written += 1

    manifest = {
        "source": str(args.columns),
        "taxonomy": str(args.taxonomy),
        "taxonomy_size": len(taxonomy),
        "n_candidate_rows": n_cand,
        "n_rows_written": n_written,
        "n_skipped_sparse": n_skipped_sparse,
        "n_types_with_rows": len(per_type_count),
        "per_type_count": dict(sorted(per_type_count.items())),
        "per_sense_origin_count": dict(sorted(per_sense_count.items())),
        "per_type_cap_hits": dict(sorted(per_type_cap_hits.items())),
        "config": {
            "ydf_confidence_floor": YDF_CONFIDENCE_FLOOR,
            "per_type_cap": PER_TYPE_CAP,
            "min_values": args.min_values,
            "seed": args.seed,
        },
    }
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"wrote {n_written:,} rows across {len(per_type_count)} YDF "
          f"labels to {args.out}", file=sys.stderr)
    print(f"manifest: {args.manifest}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
