#!/usr/bin/env python3
"""ac-01 — Extract hard negatives for the v23 precision retrain.

Reads the multi-lens diagnostic's corpus-pass output and emits a
training-data parquet covering columns where v22 fires a confident
specific Sense label that BOTH the YDF lens AND the cascade
contradict. Six target clusters from
`eval/gittables/corpus_pass/report.md` (under `reject_rate_ceil ×
misclassification`):

  rank cols     gap_id          (sense_prediction, ydf_prediction)
  ---- -------- --------------- -----------------------------------
   1   22,952   721b890ea74d…   (identity.person.gender_code,
                                 representation.discrete.categorical)
   2   21,956   1b858e0d073b…   (datetime.offset.utc,
                                 representation.numeric.integer_number)
   3    9,779   20803deffbad…   (technology.internet.url,
                                 representation.numeric.integer_number)
   4    8,649   81b63a52e3ef…   (representation.boolean.binary,
                                 representation.numeric.integer_number)
   5    5,764   cdde5d05b73a…   (datetime.component.periodicity,
                                 representation.discrete.categorical)
   6    4,835   3f2aa8465552…   (representation.identifier.alphanumeric_id,
                                 representation.discrete.categorical)

Dedup against `eval/row_hashes.tsv` per MADR 0056. A column is
excluded when ANY of its 8 sample values, hashed with the shared
normaliser, collides with an eval-corpus row hash — the corpus
column may literally be one of the eval columns under a different
path, and including it as a training row would defeat the leakage
firewall.

Outputs:
  output/v23-precision-retrain/hard_negatives.parquet
    file_path, column_name, sample_values (list<string>),
    correct_label (= ydf_prediction), cluster_id (gap_id prefix),
    sense_prediction_v22 (the FP label being trained out)
  output/v23-precision-retrain/hard_negatives_summary.md
    per-cluster row counts before/after leakage dedup

Per spec 2026-05-27-v23-precision-retrain ac-01.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
_SCRIPTS = REPO / "scripts"
if str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))

from eval_leakage import row_hash  # noqa: E402  # pyright: ignore[reportMissingImports]

CORPUS_PASS_COLUMNS = REPO / "eval/gittables/corpus_pass/columns.parquet"
EVAL_ROW_HASHES = REPO / "eval/row_hashes.tsv"
OUT_DIR = REPO / "output/v23-precision-retrain"
OUT_PARQUET = OUT_DIR / "hard_negatives.parquet"
OUT_SUMMARY = OUT_DIR / "hard_negatives_summary.md"

# (sense, ydf) pair → cluster_id (gap_id prefix). The top-6 clusters
# of report.md's `reject_rate_ceil × misclassification` cell are
# uniquely identified by their (sense, ydf) pair, so the lookup is
# unambiguous; lower-rank clusters with the same pair inherit the
# top-rank cluster_id by design (per ac-01's "filter to columns
# whose pair matches" wording).
TARGET_CLUSTERS: dict[tuple[str, str], str] = {
    ("identity.person.gender_code",
     "representation.discrete.categorical"): "721b890ea74d",
    ("datetime.offset.utc",
     "representation.numeric.integer_number"): "1b858e0d073b",
    ("technology.internet.url",
     "representation.numeric.integer_number"): "20803deffbad",
    ("representation.boolean.binary",
     "representation.numeric.integer_number"): "81b63a52e3ef",
    ("datetime.component.periodicity",
     "representation.discrete.categorical"): "cdde5d05b73a",
    ("representation.identifier.alphanumeric_id",
     "representation.discrete.categorical"): "3f2aa8465552",
}

# Corpus-pass sample_values_truncated uses the BOX DRAWINGS LIGHT
# VERTICAL `│` (U+2502) as the inter-value delimiter — chosen to
# avoid collisions with comma/pipe/tab/semicolon inside values.
SAMPLE_DELIM = "│"

MIN_REQUIRED_ROWS = 30_000  # ac-01 close evidence


def load_eval_hashes() -> set[str]:
    """Return the set of eval row_hashes from eval/row_hashes.tsv.

    Skips comment lines and the header row.
    """
    hashes: set[str] = set()
    with open(EVAL_ROW_HASHES, encoding="utf-8") as fh:
        for line in fh:
            if line.startswith("#"):
                continue
            parts = line.rstrip("\n").split("\t")
            if len(parts) != 4 or parts[3] == "row_hash":
                continue
            hashes.add(parts[3])
    return hashes


def column_collides_with_eval(column_name: str, sample_values: list[str],
                              eval_hashes: set[str]) -> bool:
    """True iff any (column_name, value) hashes match the eval set."""
    for v in sample_values:
        if not v:
            continue
        h = row_hash(column_name, v)
        if h in eval_hashes:
            return True
    return False


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns", type=Path, default=CORPUS_PASS_COLUMNS)
    p.add_argument("--out-parquet", type=Path, default=OUT_PARQUET)
    p.add_argument("--out-summary", type=Path, default=OUT_SUMMARY)
    p.add_argument("--row-hashes", type=Path, default=EVAL_ROW_HASHES)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    print(f"loading eval row hashes from {args.row_hashes}", file=sys.stderr)
    eval_hashes = load_eval_hashes()
    print(f"  {len(eval_hashes):,} eval row hashes", file=sys.stderr)

    print(f"reading {args.columns}", file=sys.stderr)
    con = duckdb.connect()

    # Compose the (sense, ydf) filter as a 6-tuple VALUES list so
    # DuckDB can run a hash join.
    pairs_sql = ", ".join(
        f"('{s}', '{y}')" for (s, y) in TARGET_CLUSTERS.keys()
    )

    arrow = con.execute(f"""
        SELECT file_path,
               column_name,
               sense_prediction,
               ydf_prediction,
               sample_values_truncated
          FROM read_parquet('{args.columns.as_posix()}')
         WHERE (sense_prediction, ydf_prediction) IN ({pairs_sql})
    """).to_arrow_table()
    n_pre = arrow.num_rows
    print(f"  {n_pre:,} columns match one of the six clusters",
          file=sys.stderr)

    # Per-cluster counts pre-dedup.
    pre_counts: dict[str, int] = {}
    rows_out: list[dict] = []
    n_collided = 0

    file_paths = arrow.column("file_path").to_pylist()
    column_names = arrow.column("column_name").to_pylist()
    senses = arrow.column("sense_prediction").to_pylist()
    ydfs = arrow.column("ydf_prediction").to_pylist()
    sample_blobs = arrow.column("sample_values_truncated").to_pylist()

    for fp, cn, sense, ydf, blob in zip(
        file_paths, column_names, senses, ydfs, sample_blobs
    ):
        cluster_id = TARGET_CLUSTERS[(sense, ydf)]
        pre_counts[cluster_id] = pre_counts.get(cluster_id, 0) + 1

        sample_values = [s for s in (blob or "").split(SAMPLE_DELIM) if s]
        if column_collides_with_eval(cn or "", sample_values, eval_hashes):
            n_collided += 1
            continue

        rows_out.append({
            "file_path": fp,
            "column_name": cn,
            "sample_values": sample_values,
            "correct_label": ydf,
            "cluster_id": cluster_id,
            "sense_prediction_v22": sense,
        })

    n_post = len(rows_out)
    print(f"  {n_collided:,} columns collide with eval row hashes "
          f"(excluded)", file=sys.stderr)
    print(f"  {n_post:,} columns kept as hard negatives", file=sys.stderr)

    if n_post < MIN_REQUIRED_ROWS:
        print(f"warn: hard-negative row count {n_post:,} is BELOW "
              f"ac-01 minimum of {MIN_REQUIRED_ROWS:,}. Writing anyway "
              f"so the gap is visible.", file=sys.stderr)

    # Per-cluster post-dedup counts.
    post_counts: dict[str, int] = {}
    for r in rows_out:
        cid = r["cluster_id"]
        post_counts[cid] = post_counts.get(cid, 0) + 1

    args.out_parquet.parent.mkdir(parents=True, exist_ok=True)

    # Write parquet via duckdb (simplest path to a typed column for
    # the list<string> sample_values).
    import pyarrow as pa  # type: ignore
    table = pa.Table.from_pylist(rows_out, schema=pa.schema([
        ("file_path", pa.string()),
        ("column_name", pa.string()),
        ("sample_values", pa.list_(pa.string())),
        ("correct_label", pa.string()),
        ("cluster_id", pa.string()),
        ("sense_prediction_v22", pa.string()),
    ]))
    import pyarrow.parquet as pq  # type: ignore
    pq.write_table(table, args.out_parquet)
    print(f"wrote {args.out_parquet}", file=sys.stderr)

    # Summary markdown.
    lines: list[str] = []
    lines.append("# v23 precision retrain — hard-negative extraction\n\n")
    lines.append("Per spec `2026-05-27-v23-precision-retrain` ac-01.\n\n")
    lines.append(
        "Hard negatives sourced from "
        "`eval/gittables/corpus_pass/columns.parquet` by filtering on "
        "the six `(sense_prediction, ydf_prediction)` pairs of the "
        "top-6 corroborated misclassification clusters. Dedup against "
        "`eval/row_hashes.tsv` per MADR 0056 — a column is excluded if "
        "ANY of its sample values, hashed with the shared normaliser, "
        "collides with the eval set.\n\n"
    )

    lines.append("## Per-cluster coverage\n\n")
    lines.append("| cluster_id | sense FP label | YDF / correct label | pre-dedup | post-dedup | dropped |\n")
    lines.append("|---|---|---|---:|---:|---:|\n")
    pair_for_cluster = {cid: pair for pair, cid in TARGET_CLUSTERS.items()}
    for cid in sorted(pre_counts, key=lambda k: -pre_counts[k]):
        sense, ydf = pair_for_cluster[cid]
        pre = pre_counts[cid]
        post = post_counts.get(cid, 0)
        dropped = pre - post
        sense_short = sense.replace("representation.", "rep.")
        ydf_short = ydf.replace("representation.", "rep.")
        lines.append(
            f"| `{cid}…` | `{sense_short}` | `{ydf_short}` | "
            f"{pre:,} | **{post:,}** | {dropped:,} |\n"
        )
    lines.append("\n")

    lines.append("## Totals\n\n")
    lines.append(f"- Columns matching one of the six pairs: {n_pre:,}\n")
    lines.append(f"- Excluded by eval row-hash collision: {n_collided:,}\n")
    lines.append(f"- **Kept as hard negatives: {n_post:,}**\n")
    lines.append(f"- ac-01 minimum: {MIN_REQUIRED_ROWS:,} — "
                 f"{'PASS' if n_post >= MIN_REQUIRED_ROWS else 'BELOW THRESHOLD'}.\n\n")

    lines.append("## Non-zero coverage check\n\n")
    missing = [cid for cid in TARGET_CLUSTERS.values()
               if post_counts.get(cid, 0) == 0]
    if missing:
        lines.append(
            f"**FAIL** — clusters with zero hard negatives after dedup: "
            f"{', '.join(missing)}\n"
        )
    else:
        lines.append("**PASS** — all six clusters have ≥1 hard negative "
                     "after dedup.\n")

    args.out_summary.write_text("".join(lines))
    print(f"wrote {args.out_summary}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
