#!/usr/bin/env python3
"""ac-03 — Mine corroborated_gaps for v22 positive training labels.

Walks `mechanism_decomposition.parquet` + `columns.parquet` to recover
every column contributing to a corroborated cluster, and emits one
training row per surviving column in the sherlock_distilled schema
(final_label, sample_values JSON, column_name). The corroborated label
is YDF's lens vote — each row carries both the cascade flag
(`mechanism_token = 'misclassification'`) and the YDF flag
(`ydf_confidence >= 0.5` and `ydf_prediction != sense_prediction`), so
each row is a per-row two-lens AND corroboration in line with the
build_corroborated_gaps clustering logic.

Clustering keys (mirrors scripts/build_corroborated_gaps.py): each row
is binned by `(criterion, mechanism, sense_prediction, shape)`, where
`shape` is the sha256 of sorted distinct char-class patterns of the
column's sample values. The per-cluster cap is then applied to the bin
before per-type caps are applied across bins.

Filters (per spec 2026-05-25-v22-boundary-training ac-03):
  - mechanism = 'misclassification' (the cascade-flag for
    recommended_action_class='training_data_addition')
  - ydf_confidence >= YDF_CONFIDENCE_FLOOR
  - ydf_prediction != sense_prediction
  - ydf_prediction in the FineType taxonomy
  - sample_values has at least --min-values entries

Caps:
  - per-cluster: 50 columns (avoids any single cluster dominating)
  - per-type:    geography.* → 3,000; everything else → 600
                 (matches v21's DOMAIN_CAP_OVERRIDES)

Output:
  output/distillation-v22/corroborated_gaps_distilled.csv.gz
  output/distillation-v22/corroborated_manifest.json
"""
from __future__ import annotations

import argparse
import csv
import gzip
import hashlib
import json
import random
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_COLUMNS = REPO / "eval/gittables/corpus_pass/columns.parquet"
DEFAULT_MECHANISM = REPO / "eval/gittables/corpus_pass/mechanism_decomposition.parquet"
DEFAULT_OUT = REPO / "output/distillation-v22/corroborated_gaps_distilled.csv.gz"
DEFAULT_MANIFEST = REPO / "output/distillation-v22/corroborated_manifest.json"
DEFAULT_TAXONOMY = REPO / "models/default/label_map.json"

PER_CLUSTER_CAP = 50
GEO_PER_TYPE_CAP = 3000
DEFAULT_PER_TYPE_CAP = 600
YDF_CONFIDENCE_FLOOR = 0.5  # matches scripts/build_corroborated_gaps.py
SAMPLE_SEPARATOR = "│"  # U+2502 — matches the corpus pass writer


def char_class(c: str) -> str:
    if "A" <= c <= "Z":
        return "A"
    if "a" <= c <= "z":
        return "a"
    if "0" <= c <= "9":
        return "9"
    return "."


def value_pattern(value: str) -> str:
    return "".join(char_class(c) for c in value)


def shape_signature(samples: list[str]) -> str:
    patterns = sorted({value_pattern(v) for v in samples if v})
    h = hashlib.sha256()
    h.update("\n".join(patterns).encode("utf-8"))
    return h.hexdigest()


def split_samples(s: str | None) -> list[str]:
    if not s:
        return []
    return [p for p in s.split(SAMPLE_SEPARATOR) if p]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns", type=Path, default=DEFAULT_COLUMNS)
    p.add_argument("--mechanism", type=Path, default=DEFAULT_MECHANISM)
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

    print("joining columns × mechanism_decomposition...", file=sys.stderr)
    con = duckdb.connect()
    con.execute("CREATE OR REPLACE TEMP TABLE taxonomy(label VARCHAR)")
    con.executemany("INSERT INTO taxonomy VALUES (?)",
                    [(t,) for t in taxonomy])

    arrow = con.execute(f"""
        SELECT m.file_path,
               m.column_name,
               m.criterion,
               m.mechanism_token   AS mechanism,
               c.sense_prediction,
               c.ydf_prediction,
               c.ydf_confidence,
               c.sample_values_truncated
          FROM read_parquet('{args.mechanism.as_posix()}') m
          JOIN read_parquet('{args.columns.as_posix()}') c
            USING (file_path, column_name)
         WHERE m.mechanism_token = 'misclassification'
           AND c.ydf_confidence >= {YDF_CONFIDENCE_FLOOR}
           AND c.ydf_prediction IS NOT NULL
           AND c.ydf_prediction <> c.sense_prediction
           AND c.ydf_prediction IN (SELECT label FROM taxonomy)
    """).to_arrow_table()
    n_joined = arrow.num_rows
    print(f"  candidate rows: {n_joined:,}", file=sys.stderr)

    cols = {n: arrow.column(n).to_pylist() for n in arrow.column_names}

    print("clustering by (criterion, mechanism, sense_pred, shape)...",
          file=sys.stderr)
    clusters: dict[tuple, list[int]] = defaultdict(list)
    samples_cache: dict[int, list[str]] = {}
    n_skipped_sparse = 0

    for i in range(n_joined):
        samples = split_samples(cols["sample_values_truncated"][i])
        if len(samples) < args.min_values:
            n_skipped_sparse += 1
            continue
        shape = shape_signature(samples)
        samples_cache[i] = samples
        key = (
            cols["criterion"][i],
            cols["mechanism"][i],
            cols["sense_prediction"][i] or "unknown",
            shape,
        )
        clusters[key].append(i)

    print(f"  clusters: {len(clusters):,}", file=sys.stderr)
    print(f"  skipped sparse: {n_skipped_sparse:,}", file=sys.stderr)

    rng = random.Random(args.seed)
    args.out.parent.mkdir(parents=True, exist_ok=True)

    per_type_count: dict[str, int] = defaultdict(int)
    per_type_cap_hits: dict[str, int] = defaultdict(int)
    per_type_clusters: dict[str, set] = defaultdict(set)
    n_written = 0

    sorted_keys = sorted(clusters.keys())
    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])
        for k_idx, key in enumerate(sorted_keys):
            indices = clusters[key]
            if len(indices) > PER_CLUSTER_CAP:
                chosen = rng.sample(indices, PER_CLUSTER_CAP)
            else:
                chosen = indices

            for i in chosen:
                ydf_pred = cols["ydf_prediction"][i]
                cap = (GEO_PER_TYPE_CAP
                       if ydf_pred.startswith("geography.")
                       else DEFAULT_PER_TYPE_CAP)
                if per_type_count[ydf_pred] >= cap:
                    per_type_cap_hits[ydf_pred] += 1
                    continue
                samples = samples_cache[i]
                column_name = cols["column_name"][i] or ""
                writer.writerow([
                    ydf_pred,
                    json.dumps(samples, ensure_ascii=False),
                    column_name,
                ])
                per_type_count[ydf_pred] += 1
                per_type_clusters[ydf_pred].add(k_idx)
                n_written += 1

    manifest = {
        "source_columns": str(args.columns),
        "source_mechanism": str(args.mechanism),
        "taxonomy": str(args.taxonomy),
        "taxonomy_size": len(taxonomy),
        "n_candidate_rows": n_joined,
        "n_clusters": len(clusters),
        "n_rows_written": n_written,
        "n_skipped_sparse": n_skipped_sparse,
        "n_types_with_rows": len(per_type_count),
        "per_type_count": dict(sorted(per_type_count.items())),
        "per_type_distinct_clusters": {
            t: len(s) for t, s in sorted(per_type_clusters.items())
        },
        "per_type_cap_hits": dict(sorted(per_type_cap_hits.items())),
        "config": {
            "per_cluster_cap": PER_CLUSTER_CAP,
            "geo_per_type_cap": GEO_PER_TYPE_CAP,
            "default_per_type_cap": DEFAULT_PER_TYPE_CAP,
            "min_values": args.min_values,
            "ydf_confidence_floor": YDF_CONFIDENCE_FLOOR,
            "seed": args.seed,
        },
    }
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"wrote {n_written:,} rows across {len(per_type_count)} types "
          f"to {args.out}", file=sys.stderr)
    print(f"manifest: {args.manifest}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
