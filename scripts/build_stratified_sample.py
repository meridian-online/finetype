#!/usr/bin/env python3
"""ac-01 (spec 2026-06-07-corpus-honest-quality-gate) — stratified corpus sampler.

The destination-drift proxy missed latdec's latitude relocation because it sampled
1,000 files uniformly: a 0.13% label landed ~18 columns, far too few to resolve a
+23% shift. The fix is not "sample more uniformly" — it is to oversample the RARE
labels so every label is legible to a 3x move, while keeping the whole sample small.

Sampling unit is the source FILE (the corpus pass profiles whole files). So we
oversample a rare label by selecting the files richest in that label until we have
enough of its columns. Common labels need no quota — they ride along in any few-
thousand-file union.

Algorithm (per-label quota with file-level selection):
  - Read the v19 baseline label distribution from the corpus-pass columns parquet.
  - For every label below --rare-floor columns corpus-wide, rank its files by how
    many columns of that label they hold, and keep files until the cumulative count
    reaches --tmin (or the label is exhausted — labels with < --tmin columns total
    are captured 100%).
  - The sample is every column in the union of kept files. Common labels (>= floor)
    are abundant in that union by construction.

The deliverable is the fixed file list (feed it to snapshot_sense_distribution.py
--file-list and to the gate scorer) plus a resolution table proving every label
retains enough columns to resolve a 3x shift. Small is only valuable if latitude is
still legible in it — the resolution table is the acceptance bar, not the size.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_BASELINE = REPO / "eval/gittables/corpus_pass/columns.parquet"
DEFAULT_OUT = REPO / "output/corpus-honest-gate"


def duck(sql: str) -> list[list[str]]:
    r = subprocess.run(["duckdb", "-noheader", "-csv", "-c", sql],
                       capture_output=True, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr)
        raise SystemExit(f"duckdb failed (exit {r.returncode})")
    rows = []
    for line in r.stdout.splitlines():
        line = line.strip()
        if line:
            rows.append([c.strip().strip('"') for c in line.split(",")])
    return rows


def keep_cte(baseline: Path, tmin: int, floor: int) -> str:
    """CTE chain ending in `keep(file_path)` — the selected file union."""
    p = baseline.as_posix()
    return f"""
WITH c AS (SELECT sense_prediction AS lbl, file_path
           FROM read_parquet('{p}')),
lab AS (SELECT lbl, COUNT(*) n FROM c GROUP BY lbl),
targets AS (SELECT lbl FROM lab WHERE n < {floor}),
fc AS (SELECT file_path, lbl, COUNT(*) k FROM c
       WHERE lbl IN (SELECT lbl FROM targets) GROUP BY file_path, lbl),
ranked AS (SELECT file_path, lbl, k,
                  SUM(k) OVER (PARTITION BY lbl ORDER BY k DESC, file_path) AS cum
           FROM fc),
keep AS (SELECT DISTINCT file_path FROM ranked WHERE cum - k < {tmin})
"""


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE,
                    help="v19 corpus-pass columns parquet defining the distribution")
    ap.add_argument("--tmin", type=int, default=400,
                    help="per-label column target for rare labels")
    ap.add_argument("--rare-floor", type=int, default=40000,
                    help="labels below this corpus-wide column count get a quota")
    ap.add_argument("--out-dir", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)

    base = args.baseline
    kc = keep_cte(base, args.tmin, args.rare_floor)

    totals = duck(f"SELECT COUNT(*), COUNT(DISTINCT file_path) "
                  f"FROM read_parquet('{base.as_posix()}');")[0]
    total_cols, total_files = int(totals[0]), int(totals[1])

    # 1. The file list.
    list_path = args.out_dir / "stratified_sample.files.txt"
    files = [r[0] for r in duck(f"{kc} SELECT file_path FROM keep ORDER BY file_path;")]
    list_path.write_text("\n".join(files) + "\n")

    # 2. Sample size.
    sz = duck(f"{kc}, samp AS (SELECT file_path FROM c WHERE file_path IN "
              f"(SELECT file_path FROM keep)) "
              f"SELECT (SELECT COUNT(*) FROM keep), (SELECT COUNT(*) FROM samp);")[0]
    n_files, n_cols = int(sz[0]), int(sz[1])

    # 3. Resolution table — full vs sample columns for every label.
    res_rows = duck(
        f"{kc}, samp AS (SELECT lbl, COUNT(*) ns FROM c "
        f"WHERE file_path IN (SELECT file_path FROM keep) GROUP BY lbl) "
        f"SELECT l.lbl, l.n, COALESCE(s.ns,0) "
        f"FROM lab l LEFT JOIN samp s USING(lbl) ORDER BY l.n DESC;")
    resolution = [
        {"label": r[0], "full_cols": int(r[1]), "sample_cols": int(r[2]),
         "sample_rate": round(int(r[2]) / int(r[1]), 4) if int(r[1]) else 0.0,
         "fully_captured": int(r[2]) == int(r[1])}
        for r in res_rows
    ]

    min_sample = min((x["sample_cols"] for x in resolution), default=0)
    # A label is "resolvable" for a 3x move if it has >= tmin cols OR is fully
    # captured (its entire tiny population is present — the honest ceiling).
    unresolved = [x for x in resolution
                  if x["sample_cols"] < args.tmin and not x["fully_captured"]]

    summary = {
        "spec": "2026-06-07-corpus-honest-quality-gate",
        "ac": "ac-01",
        "baseline": base.as_posix(),
        "params": {"tmin": args.tmin, "rare_floor": args.rare_floor},
        "corpus": {"files": total_files, "cols": total_cols},
        "sample": {
            "files": n_files, "pct_files": round(100 * n_files / total_files, 2),
            "cols": n_cols, "pct_cols": round(100 * n_cols / total_cols, 2),
        },
        "resolution": {
            "labels": len(resolution),
            "min_sample_cols": min_sample,
            "labels_fully_captured": sum(x["fully_captured"] for x in resolution),
            "labels_below_tmin_not_captured": len(unresolved),
            "unresolved": unresolved,
        },
        "file_list": list_path.as_posix(),
    }
    (args.out_dir / "stratified_sample.resolution.json").write_text(
        json.dumps({"summary": summary, "per_label": resolution}, indent=2))

    print(json.dumps(summary, indent=2))
    if unresolved:
        print(f"\nWARNING: {len(unresolved)} labels below tmin and not fully "
              f"captured — the sample cannot resolve a 3x shift on them.",
              file=sys.stderr)
    else:
        print("\nOK: every label is either >= tmin in the sample or 100% captured.",
              file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
