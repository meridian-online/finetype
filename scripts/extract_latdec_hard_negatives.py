#!/usr/bin/env python3
"""ac-02 (spec 2026-06-06-latitude-decimal-hard-negative-retrain) — extract the
latitude->decimal hard negatives for the gold-anchor family-C fix.

The single target cluster is the inverse of how v24 died. v24 pushed plain
numerics INTO geography.coordinate.latitude and exploded it 4.3x. This pushes
the FEATURE FLOATS that v19 wrongly grabs as latitude back OUT to their correct
label, representation.numeric.decimal_number:

  (sense FP label, ydf correct label)                          v19 cols
  ----------------------------------------------------------   --------
  geography.coordinate.latitude -> numeric.decimal_number       3,974

Provenance (memory corpus-pass-sense-is-v19-not-v22): the corpus pass's
sense_prediction column IS the shipped v19 default's own predictions (byte-
identical to v19_gated.parquet, distinct from v22_gated), so these 3,974 are
v19's OWN false-positives — no fresh profiling run needed.

The load-bearing difference from the v24 extractor: the (latitude, decimal) pair
is NOT a clean false-positive cluster. ~1,354 of the 3,974 are TRUE latitudes
(Lat/lat/latitude/...) that ydf merely undersells as "decimal". Training THOSE
as decimal would poison latitude RECALL — the exact price we refuse to pay
(ac-02). So a coordinate-header guard excludes any column whose header is a
latitude/longitude/projected-coordinate alias; only the genuine feature floats
(cumulative_gpa, mag, HitRate_*, velocities, RealTime(ms), LnPrior, ML min_q)
survive as hard negatives.

Deliberate deviation from the v24 recipe: NO 0.80 safety floor. The cluster's
safety_score is 0.589 (MODERATE) and safety_score is structurally BLIND to
destination drift anyway (the spec's whole premise). The real gate is the
destination-drift proxy pre-check (ac-03), not this advisory number — it is
recorded, not enforced.

Kept gates (same as v24):
  * MADR 0056 leakage dedup — a column is dropped if ANY sample value, hashed
    with row_hash(column_name, value), collides with eval/row_hashes.tsv.
  * HARD ASSERT: every emitted correct_label == decimal_number; abort otherwise.

Output:
  output/latitude-decimal-precision/hard_negatives.parquet
    file_path, column_name, sample_values (list<varchar>),
    correct_label (= decimal_number), cluster_id, sense_prediction_v19
  output/latitude-decimal-precision/hard_negatives_summary.md

Usage: scripts/extract_latdec_hard_negatives.py [--columns ...] [--out-dir ...]
"""
from __future__ import annotations

import argparse
import csv
import re
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

LAT = "geography.coordinate.latitude"
DEC = "representation.numeric.decimal_number"
CLUSTER_ID = "lat_to_dec"
SAMPLE_DELIM = "│"  # corpus inter-value delimiter (U+2502)

# Coordinate-header guard. A column whose header is any of these is a real
# geographic/projected coordinate, NOT a feature float — excluded so we never
# train a true latitude/longitude as decimal (protects coordinate RECALL).
_COORD_TOKEN_RE = re.compile(r"(^|[^a-z])(lat|latitude|lon|lng|long|longitude)([^a-z]|$)")
_COORD_ALIASES = {"x", "y", "geox", "geoy", "lat_dd", "latdd", "lon_dd", "londd"}


def is_coordinate_header(column_name: str) -> bool:
    h = (column_name or "").strip().lower()
    if h in _COORD_ALIASES:
        return True
    return bool(_COORD_TOKEN_RE.search(h))


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


def cluster_safety() -> float | None:
    """Advisory avg safety_score for (latitude, decimal) — recorded, not gated."""
    out = duckdb(
        f"SELECT avg(safety_score) "
        f"FROM read_parquet('{CORROBORATED_GAPS.as_posix()}') "
        f"WHERE sample_evidence[1].sense_prediction='{LAT}' "
        f"  AND sample_evidence[1].ydf_prediction='{DEC}';")
    val = out.strip()
    try:
        return float(val) if val else None
    except ValueError:
        return None


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
                   default=REPO / "output/latitude-decimal-precision")
    args = p.parse_args()
    out_parquet = args.out_dir / "hard_negatives.parquet"
    out_summary = args.out_dir / "hard_negatives_summary.md"
    args.out_dir.mkdir(parents=True, exist_ok=True)

    safety = cluster_safety()
    print(f"advisory safety_score (latitude->decimal): "
          f"{safety:.3f} (MODERATE — NOT gated; proxy pre-check is the gate)"
          if safety is not None else "advisory safety_score: unavailable",
          file=sys.stderr)

    eval_hashes = load_eval_hashes()
    print(f"loaded {len(eval_hashes):,} eval row hashes", file=sys.stderr)

    with tempfile.TemporaryDirectory(prefix="latdec_") as tmp:
        cand_csv = Path(tmp) / "cand.csv"
        duckdb_exec(
            f"COPY (SELECT file_path, column_name, sense_prediction, "
            f"  ydf_prediction, sample_values_truncated "
            f"FROM read_parquet('{args.columns.as_posix()}') "
            f"WHERE sense_prediction='{LAT}' AND ydf_prediction='{DEC}') "
            f"TO '{cand_csv.as_posix()}' (HEADER, FORMAT CSV);")

        n_pre = n_true_coord = n_collided = n_kept = 0
        kept_csv = Path(tmp) / "kept.csv"
        with open(cand_csv, newline="", encoding="utf-8") as fin, \
             open(kept_csv, "w", newline="", encoding="utf-8") as fout:
            w = csv.writer(fout)
            w.writerow(["file_path", "column_name", "sample_values",
                        "correct_label", "cluster_id", "sense_prediction_v19"])
            for row in csv.DictReader(fin):
                n_pre += 1
                # Coordinate-header guard: never train a real latitude/longitude
                # as decimal — that would cost coordinate recall.
                if is_coordinate_header(row["column_name"]):
                    n_true_coord += 1
                    continue
                vals = [s for s in (row["sample_values_truncated"] or "")
                        .split(SAMPLE_DELIM) if s]
                if collides(row["column_name"] or "", vals, eval_hashes):
                    n_collided += 1
                    continue
                n_kept += 1
                w.writerow([row["file_path"], row["column_name"],
                            SAMPLE_DELIM.join(vals), DEC, CLUSTER_ID,
                            row["sense_prediction"]])

        print(f"  {n_pre:,} candidates, {n_true_coord:,} true-coordinate headers "
              f"excluded, {n_collided:,} leakage-collisions dropped, "
              f"{n_kept:,} kept", file=sys.stderr)

        duckdb_exec(
            f"COPY (SELECT file_path, column_name, "
            f"  string_split(sample_values, '{SAMPLE_DELIM}') AS sample_values, "
            f"  correct_label, cluster_id, sense_prediction_v19 "
            f"FROM read_csv('{kept_csv.as_posix()}', header=true, "
            f"  all_varchar=true)) "
            f"TO '{out_parquet.as_posix()}' (FORMAT PARQUET);")

    # Post-write verification — every correct_label must be decimal_number.
    distinct_labels = duckdb(
        f"SELECT DISTINCT correct_label "
        f"FROM read_parquet('{out_parquet.as_posix()}');").split()
    bad = [lbl for lbl in distinct_labels if lbl and lbl != DEC]
    assert not bad, f"non-decimal correct_label in output: {bad}"

    lines = [
        "# latitude->decimal — hard-negative extraction\n\n",
        "Per spec `2026-06-06-latitude-decimal-hard-negative-retrain` ac-02. "
        "The inverse of v24's failure: feature floats v19 wrongly grabs as "
        "latitude, pushed back to decimal_number.\n\n",
        "## Coverage\n\n",
        "| cluster_id | sense FP label | correct label | advisory safety | "
        "candidates | true-coord excluded | leakage dropped | kept |\n",
        "|---|---|---|---:|---:|---:|---:|---:|\n",
        f"| `{CLUSTER_ID}` | `{LAT}` | `{DEC.replace('representation.','rep.')}` "
        f"| {safety:.3f} | {n_pre:,} | {n_true_coord:,} | {n_collided:,} | "
        f"**{n_kept:,}** |\n" if safety is not None else
        f"| `{CLUSTER_ID}` | `{LAT}` | `{DEC.replace('representation.','rep.')}` "
        f"| n/a | {n_pre:,} | {n_true_coord:,} | {n_collided:,} | "
        f"**{n_kept:,}** |\n",
        "\n## Gates\n\n",
        "- Safety floor: NONE. safety_score is advisory only and structurally "
        "blind to destination drift; the proxy pre-check (ac-03) is the gate.\n",
        f"- Coordinate-header guard: {n_true_coord:,} columns excluded as real "
        "latitude/longitude/projected coordinates (protects coordinate recall — "
        "ac-02's invariant).\n",
        f"- Leakage dedup (MADR 0056): {n_collided:,} columns dropped on eval "
        "row-hash collision.\n",
        f"- correct_label set = `{sorted(set(distinct_labels))}` — decimal only: "
        "**PASS** (asserted).\n",
        f"\n## Total kept: {n_kept:,}\n",
    ]
    out_summary.write_text("".join(lines))

    print(f"wrote {out_parquet}\nwrote {out_summary}", file=sys.stderr)
    print(f"labels in output: {sorted(set(distinct_labels))}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
