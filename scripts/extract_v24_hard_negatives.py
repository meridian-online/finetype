#!/usr/bin/env python3
"""v24 ac-01 — extract hard negatives for the FOUR HIGH-safety numeric-target
clusters ONLY. Deliberately NOT v23's mix: every categorical-target cluster is
excluded, because additive training on discrete.categorical is exactly what
sank v23 (it pulled ~48k geography columns into categorical, +529.6%).

Targets (re-baselined live on the shipped v19 default in ac-00 —
output/v24-numeric-precision/rebaseline_v19.md — all four survive):

  (sense FP label, ydf correct label)                          safety  v19 FP
  ----------------------------------------------------------   ------  ------
  datetime.offset.utc            -> numeric.integer_number      0.95   0.837
  representation.boolean.binary  -> numeric.integer_number      0.94   0.993
  technology.internet.url        -> numeric.integer_number      0.91   1.000
  numeric.integer_number         -> numeric.decimal_number      0.84   1.000

Gates (ac-01):
  * cluster safety_score >= 0.80, read live from corroborated_gaps.parquet
    (averaged over the gap rows for each (sense, ydf) pair). A cluster below
    threshold is dropped with a warning.
  * MADR 0056 leakage dedup — a column is excluded if ANY of its sample values,
    hashed with the shared normaliser row_hash(column_name, value), collides
    with eval/row_hashes.tsv.
  * HARD ASSERT: no emitted row carries correct_label = discrete.categorical.
    The script aborts non-zero if one ever would (guards against a stray pair).

Output:
  output/v24-numeric-precision/hard_negatives.parquet
    file_path, column_name, sample_values (list<varchar>),
    correct_label (= ydf numeric label), cluster_id, sense_prediction_v24
  output/v24-numeric-precision/hard_negatives_summary.md

No python duckdb/pyarrow modules required — parquet I/O goes through the duckdb
CLI; only hashing/filtering is in python.

Usage: scripts/extract_v24_hard_negatives.py [--columns ...] [--out-dir ...]
"""
from __future__ import annotations

import argparse
import csv
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
_SCRIPTS = REPO / "scripts"
if str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))

from eval_leakage import row_hash  # noqa: E402  # pyright: ignore[reportMissingImports]

CORPUS_COLUMNS = REPO / "eval/gittables/corpus_pass/columns.parquet"
CORROBORATED_GAPS = REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet"
EVAL_ROW_HASHES = REPO / "eval/row_hashes.tsv"

INT = "representation.numeric.integer_number"
DEC = "representation.numeric.decimal_number"
CATEGORICAL = "representation.discrete.categorical"

# (sense_fp, ydf_correct) -> short cluster_id. Numeric-target only.
TARGET_CLUSTERS: dict[tuple[str, str], str] = {
    ("datetime.offset.utc", INT): "utc_to_int",
    ("representation.boolean.binary", INT): "bool_to_int",
    ("technology.internet.url", INT): "url_to_int",
    (INT, DEC): "int_to_dec",
}

SAFETY_FLOOR = 0.80
SAMPLE_DELIM = "│"  # corpus inter-value delimiter (U+2502)


def duckdb(sql: str) -> str:
    r = subprocess.run(["duckdb", "-noheader", "-csv", "-c", sql],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(f"duckdb error:\n{r.stderr}", file=sys.stderr)
        sys.exit(2)
    return r.stdout


def duckdb_exec(sql: str) -> None:
    r = subprocess.run(["duckdb", "-c", sql], capture_output=True, text=True)
    if r.returncode != 0:
        print(f"duckdb error:\n{r.stderr}", file=sys.stderr)
        sys.exit(2)


def cluster_safety() -> dict[tuple[str, str], float]:
    """Per-(sense,ydf) avg safety_score from the corroborated gaps."""
    pairs_sql = ", ".join(f"('{s}','{y}')" for (s, y) in TARGET_CLUSTERS)
    out = duckdb(
        f"SELECT sample_evidence[1].sense_prediction, "
        f"       sample_evidence[1].ydf_prediction, "
        f"       avg(safety_score) "
        f"FROM read_parquet('{CORROBORATED_GAPS.as_posix()}') "
        f"WHERE criterion='reject_rate_ceil' AND mechanism='misclassification' "
        f"  AND (sample_evidence[1].sense_prediction, "
        f"       sample_evidence[1].ydf_prediction) IN ({pairs_sql}) "
        f"GROUP BY 1,2;")
    safety: dict[tuple[str, str], float] = {}
    for line in out.splitlines():
        parts = next(csv.reader([line]))
        if len(parts) != 3:
            continue
        safety[(parts[0], parts[1])] = float(parts[2])
    return safety


def load_eval_hashes() -> set[str]:
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


def collides(column_name: str, values: list[str], eval_hashes: set[str]) -> bool:
    for v in values:
        if v and row_hash(column_name, v) in eval_hashes:
            return True
    return False


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns", type=Path, default=CORPUS_COLUMNS)
    p.add_argument("--out-dir", type=Path,
                   default=REPO / "output/v24-numeric-precision")
    args = p.parse_args()
    out_parquet = args.out_dir / "hard_negatives.parquet"
    out_summary = args.out_dir / "hard_negatives_summary.md"
    args.out_dir.mkdir(parents=True, exist_ok=True)

    # 1. Safety gate — drop any target cluster below the floor.
    safety = cluster_safety()
    active: dict[tuple[str, str], str] = {}
    dropped_for_safety: list[tuple[str, str, float]] = []
    for pair, cid in TARGET_CLUSTERS.items():
        s = safety.get(pair, 0.0)
        if s >= SAFETY_FLOOR:
            active[pair] = cid
        else:
            dropped_for_safety.append((pair[0], pair[1], s))
            print(f"warn: cluster {cid} safety {s:.2f} < {SAFETY_FLOOR} "
                  f"— dropped", file=sys.stderr)
    if not active:
        print("error: no clusters pass the safety floor", file=sys.stderr)
        return 2

    # 2. Pull candidate columns for the active clusters via the duckdb CLI.
    eval_hashes = load_eval_hashes()
    print(f"loaded {len(eval_hashes):,} eval row hashes", file=sys.stderr)

    pairs_sql = ", ".join(f"('{s}','{y}')" for (s, y) in active)
    with tempfile.TemporaryDirectory(prefix="v24hn_") as tmp:
        cand_csv = Path(tmp) / "cand.csv"
        duckdb_exec(
            f"COPY (SELECT file_path, column_name, sense_prediction, "
            f"  ydf_prediction, sample_values_truncated "
            f"FROM read_parquet('{args.columns.as_posix()}') "
            f"WHERE (sense_prediction, ydf_prediction) IN ({pairs_sql})) "
            f"TO '{cand_csv.as_posix()}' (HEADER, FORMAT CSV);")

        pre: dict[str, int] = {}
        post: dict[str, int] = {}
        n_pre = n_collided = 0
        kept_csv = Path(tmp) / "kept.csv"
        with open(cand_csv, newline="", encoding="utf-8") as fin, \
             open(kept_csv, "w", newline="", encoding="utf-8") as fout:
            w = csv.writer(fout)
            w.writerow(["file_path", "column_name", "sample_values",
                        "correct_label", "cluster_id", "sense_prediction_v24"])
            for row in csv.DictReader(fin):
                sense = row["sense_prediction"]
                ydf = row["ydf_prediction"]
                cid = active.get((sense, ydf))
                if cid is None:
                    continue
                n_pre += 1
                pre[cid] = pre.get(cid, 0) + 1

                # HARD guard: never emit a categorical correct_label.
                if ydf == CATEGORICAL:
                    print(f"error: categorical correct_label reached emit path "
                          f"for cluster {cid} — aborting", file=sys.stderr)
                    return 3

                vals = [s for s in (row["sample_values_truncated"] or "")
                        .split(SAMPLE_DELIM) if s]
                if collides(row["column_name"] or "", vals, eval_hashes):
                    n_collided += 1
                    continue
                post[cid] = post.get(cid, 0) + 1
                w.writerow([row["file_path"], row["column_name"],
                            SAMPLE_DELIM.join(vals), ydf, cid, sense])

        n_post = sum(post.values())
        print(f"  {n_pre:,} candidates, {n_collided:,} leakage-collisions "
              f"dropped, {n_post:,} kept", file=sys.stderr)

        # 3. CSV -> parquet (list<varchar> sample_values) via the duckdb CLI.
        duckdb_exec(
            f"COPY (SELECT file_path, column_name, "
            f"  string_split(sample_values, '{SAMPLE_DELIM}') AS sample_values, "
            f"  correct_label, cluster_id, sense_prediction_v24 "
            f"FROM read_csv('{kept_csv.as_posix()}', header=true, "
            f"  all_varchar=true)) "
            f"TO '{out_parquet.as_posix()}' (FORMAT PARQUET);")

    # 4. Post-write verification straight off the parquet.
    distinct_labels = duckdb(
        f"SELECT DISTINCT correct_label "
        f"FROM read_parquet('{out_parquet.as_posix()}');").split()
    cat_rows = duckdb(
        f"SELECT count(*) FROM read_parquet('{out_parquet.as_posix()}') "
        f"WHERE correct_label='{CATEGORICAL}';").strip()
    assert cat_rows == "0", f"categorical rows present in output: {cat_rows}"

    # 5. Summary.
    lines = ["# v24 numeric-precision — hard-negative extraction\n\n",
             "Per spec `2026-06-03-v24-numeric-precision` ac-01. Numeric-target "
             "clusters ONLY; every categorical-target cluster is excluded by "
             "construction (the v23 failure mode).\n\n",
             "## Per-cluster coverage\n\n",
             "| cluster_id | sense FP label | ydf correct | safety | pre-dedup | "
             "kept | dropped |\n",
             "|---|---|---|---:|---:|---:|---:|\n"]
    pair_for_cid = {cid: pair for pair, cid in active.items()}
    for cid in sorted(post, key=lambda k: -post[k]):
        sense, ydf = pair_for_cid[cid]
        pr, po = pre.get(cid, 0), post.get(cid, 0)
        lines.append(
            f"| `{cid}` | `{sense}` | `{ydf.replace('representation.','rep.')}` | "
            f"{safety[(sense, ydf)]:.2f} | {pr:,} | **{po:,}** | {pr-po:,} |\n")
    lines.append("\n## Gates\n\n")
    lines.append(f"- Safety floor {SAFETY_FLOOR}: "
                 f"{'all clusters pass' if not dropped_for_safety else dropped_for_safety}\n")
    lines.append(f"- Leakage dedup (MADR 0056): {n_collided:,} columns dropped "
                 f"on eval row-hash collision.\n")
    zero = [cid for cid in active.values() if post.get(cid, 0) == 0]
    lines.append(f"- Non-zero coverage on all four: "
                 f"{'**PASS**' if not zero else f'**FAIL** — empty: {zero}'}\n")
    lines.append(f"- correct_label set = `{sorted(set(distinct_labels))}` — "
                 f"zero categorical: **PASS** (asserted).\n")
    lines.append(f"\n## Total kept: {n_post:,}\n")
    out_summary.write_text("".join(lines))

    print(f"wrote {out_parquet}\nwrote {out_summary}", file=sys.stderr)
    print(f"labels in output: {sorted(set(distinct_labels))}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
