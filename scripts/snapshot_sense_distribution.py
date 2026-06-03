#!/usr/bin/env python3
"""v24 ac-02 / ac-04 — pre/post Sense-distribution snapshot.

The mandatory failure-mode guard (CLAUDE.md): a training_data_addition retrain
must show NO categorical explosion and NO geography regression vs the pre-train
default — the precise way v23 died. This script captures the label -> column
histogram for ONE model so the pre-train (v19) and post-train (v24) snapshots
can be diffed by identical code.

Sampling unit is the source FILE (a random sample of distinct file_paths from
the corpus), profiled whole — many columns per profile call, so a representative
label histogram is cheap. Same --seed gives the same file set across models, so
pre vs post is an apples-to-apples diff.

The `watch` block surfaces exactly the v23 failure signals plus the v24 targets:
  - categorical_pct          (v23 exploded this: +529.6%)
  - geography {city,region,country} counts (v23 regressed these)
  - the four v24 FP labels as Sense outputs (utc/url/boolean/integer) — these
    should FALL post-train if v24 works.

Usage:
  FINETYPE_MODEL=models/sherlock-v19-relu-s42 \
    scripts/snapshot_sense_distribution.py --label v19 [--files 800] [--seed 42]
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BIN = os.environ.get("FINETYPE_BIN", str(REPO / "target/release/finetype"))
MODEL = os.environ.get("FINETYPE_MODEL", "models/sherlock-v19-relu-s42")
CORPUS_COLUMNS = REPO / "eval/gittables/corpus_pass/columns.parquet"

CATEGORICAL = "representation.discrete.categorical"
GEO = {
    "city": "geography.location.city",
    "region": "geography.location.region",
    "country": "geography.location.country",
}
# v24 FP labels (as Sense outputs) — should fall post-train.
V24_FP = {
    "utc": "datetime.offset.utc",
    "url": "technology.internet.url",
    "boolean": "representation.boolean.binary",
    "integer": "representation.numeric.integer_number",
}


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def sample_files(columns_pq: Path, n: int, seed: int) -> list[str]:
    r = run(["duckdb", "-noheader", "-csv", "-c",
             f"SELECT file_path FROM ("
             f"  SELECT DISTINCT file_path "
             f"  FROM read_parquet('{columns_pq.as_posix()}')"
             f") USING SAMPLE {n} ROWS (reservoir, {seed});"])
    return [l.strip().strip('"') for l in r.stdout.splitlines() if l.strip()]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--label", required=True, help="snapshot label, e.g. v19")
    ap.add_argument("--files", type=int, default=800)
    ap.add_argument("--rows", type=int, default=1000)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--columns", type=Path, default=CORPUS_COLUMNS)
    ap.add_argument("--out-dir", type=Path,
                    default=REPO / "output/v24-numeric-precision")
    args = ap.parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)

    if not os.path.isdir(MODEL):
        print(f"error: model dir not found: {MODEL}", file=sys.stderr)
        return 3

    files = sample_files(args.columns, args.files, args.seed)
    print(f"sampled {len(files)} source files (seed {args.seed})",
          file=sys.stderr)

    with tempfile.TemporaryDirectory(prefix="sensedist_") as work:
        csvdir = Path(work) / "csv"
        schemadir = Path(work) / "schema"
        csvdir.mkdir()
        schemadir.mkdir()

        idx_paths = []
        convert_err = 0
        for i, f in enumerate(files):
            if not os.path.isfile(f):
                convert_err += 1
                continue
            csvp = csvdir / f"{i:05d}.csv"
            c = run(["duckdb", "-c",
                     f"COPY (SELECT * FROM read_parquet('{os.path.realpath(f)}') "
                     f"LIMIT {args.rows}) TO '{csvp.as_posix()}' "
                     f"(HEADER, FORMAT CSV);"])
            if c.returncode != 0 or not csvp.is_file():
                convert_err += 1
                continue
            idx_paths.append(str(csvp))

        listfile = Path(work) / "list.txt"
        listfile.write_text("\n".join(idx_paths) + "\n")

        r = run([BIN, "profile", "--files", str(listfile),
                 "--out-dir", str(schemadir), "-o", "json-schema"],
                env={**os.environ, "FINETYPE_MODEL": MODEL})
        if r.returncode != 0:
            print(f"profile non-zero exit; stderr tail:\n{r.stderr[-300:]}",
                  file=sys.stderr)

        counts: dict[str, int] = {}
        files_profiled = 0
        for schema in schemadir.glob("*.json"):
            try:
                sj = json.load(open(schema))
            except Exception:
                continue
            files_profiled += 1
            for spec in sj.get("properties", {}).values():
                label = spec.get("x-finetype-label", "")
                counts[label] = counts.get(label, 0) + 1

    total = sum(counts.values())
    watch = {
        "categorical_count": counts.get(CATEGORICAL, 0),
        "categorical_pct": round(counts.get(CATEGORICAL, 0) / total, 4) if total else 0.0,
        "geography": {k: counts.get(v, 0) for k, v in GEO.items()},
        "v24_fp_labels": {k: counts.get(v, 0) for k, v in V24_FP.items()},
    }
    out = {
        "label": args.label,
        "model": MODEL,
        "files_sampled": len(files),
        "files_profiled": files_profiled,
        "convert_err": convert_err,
        "total_cols": total,
        "seed": args.seed,
        "watch": watch,
        "label_counts": dict(sorted(counts.items(), key=lambda kv: -kv[1])),
    }
    out_path = args.out_dir / f"sense_dist_{args.label}.json"
    out_path.write_text(json.dumps(out, indent=2))
    print(json.dumps({"label": args.label, "total_cols": total,
                      "files_profiled": files_profiled, "watch": watch},
                     indent=2))
    print(f"wrote {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
