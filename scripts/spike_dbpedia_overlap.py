#!/usr/bin/env python3
"""DBpedia / Schema.org overlap spike for gittables multi-lens diagnostic.

Spec: `.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/spec.yaml`
ac-02 (DBpedia overlap spike).

Sample:
  One lex-first parquet per topic directory under
  /Users/hugh/datasets/gittables/ (94 topic dirs → 94 tables).
  Sampling within each topic is deterministic — the
  lexicographically-first parquet.

Extraction:
  For each sample parquet, read KV metadata under the `gittables` key
  via DuckDB's `parquet_kv_metadata()`. Parse JSON. Extract the
  per-column entries from `dbpedia_semantic_column_types` and
  `schema_semantic_column_types`.

Output (Phase 1 — raw):
  - eval/gittables/dbpedia_overlap_spike_raw.tsv:
    (topic, table, column, dbpedia_class, schema_class)
  - eval/gittables/dbpedia_overlap_spike_universe.tsv:
    (annotation_id, count, namespace) — distinct DBpedia / Schema.org
    classes appearing in the sample with their column counts.

The mapping check (column annotation → non-trivial FineType type) and
the final spike JSON (`dbpedia_overlap_spike.json`) are produced by a
follow-up step that consumes the universe TSV after hand-mapping.

Usage:
  python3 scripts/spike_dbpedia_overlap.py [--corpus-root <path>]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

DEFAULT_CORPUS_ROOT = Path("/Users/hugh/datasets/gittables")
DEFAULT_OUT_DIR = Path("eval/gittables")
DUCKDB_BIN = "duckdb"


@dataclass(frozen=True)
class ColumnAnnotation:
    topic: str
    table: str
    column: str
    dbpedia_class: str | None
    schema_class: str | None


def _list_topics(corpus_root: Path) -> list[Path]:
    return sorted(p for p in corpus_root.iterdir() if p.is_dir())


def _lex_first_parquet(topic_dir: Path) -> Path | None:
    candidates = sorted(topic_dir.glob("*.parquet"))
    return candidates[0] if candidates else None


def _read_kv_blob(parquet: Path) -> dict | None:
    sql = (
        "SELECT decode(value) FROM parquet_kv_metadata("
        f"'{str(parquet).replace(chr(39), chr(39) * 2)}'"
        ") WHERE decode(key)='gittables';"
    )
    res = subprocess.run(
        [DUCKDB_BIN, "-noheader", "-list", "-c", sql],
        capture_output=True, text=True, timeout=30,
    )
    if res.returncode != 0:
        print(
            f"warn: duckdb failed on {parquet.name}: "
            f"{res.stderr.strip()[-200:]}",
            file=sys.stderr,
        )
        return None
    blob = res.stdout.strip()
    if not blob:
        return None
    try:
        return json.loads(blob)
    except json.JSONDecodeError as exc:
        print(
            f"warn: bad JSON in {parquet.name}: {exc}",
            file=sys.stderr,
        )
        return None


def _extract_per_column(
    topic: str, table: str, kv: dict
) -> list[ColumnAnnotation]:
    dbpedia = kv.get("dbpedia_semantic_column_types") or {}
    schema = kv.get("schema_semantic_column_types") or {}
    columns = set(dbpedia) | set(schema) | set(kv.get("dtypes") or {})

    out: list[ColumnAnnotation] = []
    for col in sorted(columns):
        d = dbpedia.get(col)
        s = schema.get(col)
        d_id = d.get("id") if isinstance(d, dict) else None
        s_id = s.get("id") if isinstance(s, dict) else None
        out.append(ColumnAnnotation(
            topic=topic, table=table, column=col,
            dbpedia_class=d_id, schema_class=s_id,
        ))
    return out


def main() -> int:
    parser = argparse.ArgumentParser(
        description="DBpedia / Schema.org overlap spike — gittables ac-02"
    )
    parser.add_argument(
        "--corpus-root", type=Path, default=DEFAULT_CORPUS_ROOT,
        help="Root containing per-topic gittables directories.",
    )
    parser.add_argument(
        "--out-dir", type=Path, default=DEFAULT_OUT_DIR,
        help="Output directory for raw + universe TSVs.",
    )
    args = parser.parse_args()

    if not args.corpus_root.exists():
        print(f"error: corpus root not found: {args.corpus_root}",
              file=sys.stderr)
        return 2

    topics = _list_topics(args.corpus_root)
    if not topics:
        print(f"error: no topic dirs under {args.corpus_root}",
              file=sys.stderr)
        return 2
    print(f"sampling: {len(topics)} topic directories", file=sys.stderr)

    sample: list[ColumnAnnotation] = []
    sampled_files = 0
    failed_files = 0
    for topic_dir in topics:
        parquet = _lex_first_parquet(topic_dir)
        if parquet is None:
            print(f"warn: no parquet in topic {topic_dir.name}",
                  file=sys.stderr)
            failed_files += 1
            continue
        kv = _read_kv_blob(parquet)
        if kv is None:
            failed_files += 1
            continue
        sample.extend(_extract_per_column(
            topic=topic_dir.name, table=parquet.name, kv=kv,
        ))
        sampled_files += 1
        if sampled_files % 10 == 0:
            print(f"  {sampled_files}/{len(topics)} parquets read",
                  file=sys.stderr)

    print(
        f"done: {sampled_files} parquets read, {failed_files} failed, "
        f"{len(sample)} column rows",
        file=sys.stderr,
    )

    args.out_dir.mkdir(parents=True, exist_ok=True)

    raw_path = args.out_dir / "dbpedia_overlap_spike_raw.tsv"
    with raw_path.open("w") as fh:
        fh.write("topic\ttable\tcolumn\tdbpedia_class\tschema_class\n")
        for ann in sample:
            col_safe = (ann.column or "").replace("\t", " ").replace("\n", " ")
            fh.write(
                f"{ann.topic}\t{ann.table}\t{col_safe}\t"
                f"{ann.dbpedia_class or ''}\t{ann.schema_class or ''}\n"
            )
    print(f"wrote {raw_path}", file=sys.stderr)

    dbpedia_counts: Counter[str] = Counter()
    schema_counts: Counter[str] = Counter()
    for ann in sample:
        if ann.dbpedia_class:
            dbpedia_counts[ann.dbpedia_class] += 1
        if ann.schema_class:
            schema_counts[ann.schema_class] += 1

    universe_path = args.out_dir / "dbpedia_overlap_spike_universe.tsv"
    with universe_path.open("w") as fh:
        fh.write("annotation_id\tcount\tnamespace\n")
        for cls, n in sorted(dbpedia_counts.items(),
                             key=lambda x: (-x[1], x[0])):
            fh.write(f"{cls}\t{n}\tdbpedia\n")
        for cls, n in sorted(schema_counts.items(),
                             key=lambda x: (-x[1], x[0])):
            fh.write(f"{cls}\t{n}\tschema\n")
    print(f"wrote {universe_path}", file=sys.stderr)

    n_cols_total = len(sample)
    n_with_dbpedia = sum(1 for a in sample if a.dbpedia_class)
    n_with_schema = sum(1 for a in sample if a.schema_class)
    n_with_either = sum(
        1 for a in sample if a.dbpedia_class or a.schema_class
    )
    summary = {
        "sampled_files": sampled_files,
        "failed_files": failed_files,
        "n_topic_dirs": len(topics),
        "n_columns_total": n_cols_total,
        "n_columns_with_dbpedia_annot": n_with_dbpedia,
        "n_columns_with_schema_annot": n_with_schema,
        "n_columns_with_either_annot": n_with_either,
        "n_distinct_dbpedia_classes": len(dbpedia_counts),
        "n_distinct_schema_classes": len(schema_counts),
    }
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
